use std::{
    path::PathBuf,
    ptr,
    sync::{Arc, Mutex},
};

use anyhow::{Context, Result};
use tao::window::Window;
use tracing::{error, info};

use cef::wrapper::message_router::{
    BrowserSideCallback, BrowserSideRouter, MessageRouterBrowserSide,
};
use cef::{ImplBrowser as _, ImplBrowserHost as _};

use crate::desktop_bridge::{BridgeOriginPolicy, NavigationPolicy};

use super::state::{
    SharedAppSchemeState, SharedBridgeIpcState, SharedBrowserState, SharedMessagePumpState,
};
use super::{
    CefariApp, CefariBridgeIpcHandler, CefariCefClient, bridge_router_config,
    browser_bounds_for_window, cef_settings, configure_cef_api_version, log_cef_runtime_paths,
    native_window_handle, prepare_cef_runtime_dirs,
};
use crate::desktop_cef::paths::{CefRuntimePathConfig, resolve_cef_runtime_paths};

pub struct CefRuntime {
    initialized: bool,
    #[cfg(target_os = "macos")]
    #[allow(dead_code)]
    library_loader: CefLibraryLoader,
    #[allow(dead_code)]
    state: SharedBrowserState,
    #[allow(dead_code)]
    app: cef::App,
    client: cef::Client,
    bridge_ipc: SharedBridgeIpcState,
    app_scheme: SharedAppSchemeState,
    message_pump: SharedMessagePumpState,
}

pub type CefBridgeIpcCallback = Arc<Mutex<dyn BrowserSideCallback>>;

pub struct CefBridgeIpcRequest {
    pub origin: String,
    pub request_json: String,
    pub callback: CefBridgeIpcCallback,
}

impl std::fmt::Debug for CefBridgeIpcRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CefBridgeIpcRequest")
            .field("origin", &self.origin)
            .field("request_json", &self.request_json)
            .finish_non_exhaustive()
    }
}

pub trait BridgeIpcSender: Send + Sync {
    fn send_bridge_ipc(&self, request: CefBridgeIpcRequest) -> Result<()>;
}

pub trait MessagePumpScheduler: Send + Sync {
    fn schedule_message_pump_work(&self, delay_ms: i64) -> Result<()>;
}

#[cfg(target_os = "macos")]
type CefLibraryLoader = cef::library_loader::LibraryLoader;

#[cfg(target_os = "macos")]
fn load_cef_library(runtime_paths: &CefRuntimePathConfig) -> Result<CefLibraryLoader> {
    let framework_dir = runtime_paths
        .framework_dir_path
        .as_ref()
        .context("CEF framework directory was not found")?;
    let loader_exe = crate::desktop_cef::paths::macos_loader_executable(
        &runtime_paths.root_cache_path,
        &runtime_paths.executable_path,
    );
    let helper_process =
        crate::desktop_cef::paths::macos_is_helper_executable(&runtime_paths.executable_path);
    let loader_macos_dir = loader_exe
        .parent()
        .context("CEF loader executable path has no parent directory")?;
    let loader_frameworks_dir = crate::desktop_cef::paths::macos_frameworks_dir(
        &runtime_paths.root_cache_path,
        &runtime_paths.executable_path,
    );

    if !helper_process {
        std::fs::create_dir_all(loader_macos_dir).with_context(|| {
            format!(
                "failed to create CEF loader layout at {}",
                loader_macos_dir.display()
            )
        })?;
        crate::desktop_cef::macos_helpers::create_clean_directory(&loader_frameworks_dir)?;
        crate::desktop_cef::macos_helpers::replace_symlink(
            &loader_frameworks_dir.join("Chromium Embedded Framework.framework"),
            framework_dir,
        )?;
        crate::desktop_cef::macos_helpers::prepare_macos_helper_apps(
            runtime_paths,
            &loader_frameworks_dir,
        )?;
    }

    let loader = CefLibraryLoader::new(&loader_exe, helper_process);
    if !loader.load() {
        anyhow::bail!(
            "failed to load CEF framework from {}",
            framework_dir.join("Chromium Embedded Framework").display()
        );
    }
    info!(
        framework = %framework_dir.display(),
        "CEF framework loaded"
    );
    Ok(loader)
}

#[allow(dead_code)]
impl CefRuntime {
    pub fn initialize(paths: &cefari_core::RuntimePaths) -> Result<Self> {
        let args = cef::args::Args::new();
        let runtime_paths = resolve_cef_runtime_paths(paths);
        prepare_cef_runtime_dirs(&runtime_paths)?;
        #[cfg(target_os = "macos")]
        let library_loader = load_cef_library(&runtime_paths)?;
        configure_cef_api_version()?;
        let router_config = bridge_router_config();
        let message_pump = SharedMessagePumpState::default();
        let mut app = CefariApp::build(router_config.clone(), message_pump.clone());
        let subprocess_exit =
            cef::execute_process(Some(args.as_main_args()), Some(&mut app), ptr::null_mut());

        if subprocess_exit >= 0 {
            info!(status = subprocess_exit, "CEF subprocess completed");
            std::process::exit(subprocess_exit);
        }

        let settings = cef_settings(&runtime_paths);
        log_cef_runtime_paths(&runtime_paths);

        let initialized = cef::initialize(
            Some(args.as_main_args()),
            Some(&settings),
            Some(&mut app),
            ptr::null_mut(),
        );

        if initialized != 1 {
            error!(
                initialized,
                "CEF initialization failed before browser creation"
            );
            anyhow::bail!("CEF initialization returned {initialized}");
        }

        let state = SharedBrowserState::default();
        let bridge_ipc = SharedBridgeIpcState::default();
        let app_scheme = SharedAppSchemeState::default();
        let bridge_origin_policy = BridgeOriginPolicy::from_environment();
        let navigation_policy = NavigationPolicy::new(bridge_origin_policy.clone());
        let browser_router = <BrowserSideRouter as MessageRouterBrowserSide>::new(router_config);
        browser_router.add_handler(
            Arc::new(CefariBridgeIpcHandler::new(
                bridge_ipc.clone(),
                bridge_origin_policy.clone(),
            )),
            false,
        );

        info!("CEF initialized");
        Ok(Self {
            initialized: true,
            #[cfg(target_os = "macos")]
            library_loader,
            state: state.clone(),
            app,
            client: CefariCefClient::build(
                state,
                bridge_origin_policy,
                navigation_policy,
                browser_router,
                app_scheme.clone(),
            ),
            bridge_ipc,
            app_scheme,
            message_pump,
        })
    }

    pub fn create_browser(&mut self, window: &Window, url: &str) -> Result<()> {
        if !self.initialized {
            anyhow::bail!("CEF is not initialized");
        }

        let handle = native_window_handle(window)?;
        let bounds = browser_bounds_for_window(window);
        let window_info = cef::WindowInfo::default().set_as_child(handle, &bounds);
        let settings = cef::BrowserSettings::default();
        let url = cef::CefString::from(url);
        let created = cef::browser_host_create_browser(
            Some(&window_info),
            Some(&mut self.client),
            Some(&url),
            Some(&settings),
            None,
            None,
        );

        if created != 1 {
            error!(
                created,
                url = %url.to_string(),
                width = bounds.width,
                height = bounds.height,
                "CEF browser creation failed"
            );
            anyhow::bail!("CEF browser creation returned {created}");
        }

        info!("CEF browser created");
        Ok(())
    }

    pub fn has_browser(&self) -> bool {
        self.state.has_browser()
    }

    pub fn browser_identifier(&self) -> Result<i32> {
        self.state
            .active_browser()
            .map(|browser| browser.identifier())
    }

    pub fn reload_browser(&self) -> Result<()> {
        let browser = self.state.active_browser()?;
        browser.reload();
        Ok(())
    }

    pub fn focus_browser(&self, focused: bool) -> Result<()> {
        let host = self.browser_host()?;
        host.set_focus(i32::from(focused));
        Ok(())
    }

    pub fn open_dev_tools(&self) -> Result<()> {
        let host = self.browser_host()?;
        if host.has_dev_tools() != 0 {
            return Ok(());
        }

        let window_info = cef::WindowInfo::default();
        let settings = cef::BrowserSettings::default();
        host.show_dev_tools(Some(&window_info), None, Some(&settings), None);
        Ok(())
    }

    pub fn close_browser(&self, force_close: bool) -> Result<()> {
        let host = self.browser_host()?;
        host.close_browser(i32::from(force_close));
        Ok(())
    }

    pub fn notify_browser_resized(&self) -> Result<()> {
        let host = self.browser_host()?;
        host.was_resized();
        Ok(())
    }

    pub fn notify_browser_screen_info_changed(&self) -> Result<()> {
        let host = self.browser_host()?;
        host.notify_screen_info_changed();
        Ok(())
    }

    pub fn notify_browser_move_or_resize_started(&self) -> Result<()> {
        let host = self.browser_host()?;
        host.notify_move_or_resize_started();
        Ok(())
    }

    pub fn pump_message_loop(&self) {
        if self.initialized {
            cef::do_message_loop_work();
        }
    }

    pub fn set_bridge_ipc_sender(&self, sender: Arc<dyn BridgeIpcSender>) {
        self.bridge_ipc.set_sender(sender);
    }

    pub fn set_app_scheme_resource_dir(&self, resource_dir: PathBuf) {
        self.app_scheme.set_resource_dir(resource_dir);
    }

    pub fn set_message_pump_scheduler(&self, scheduler: Arc<dyn MessagePumpScheduler>) {
        self.message_pump.set_scheduler(scheduler);
    }

    fn browser_host(&self) -> Result<cef::BrowserHost> {
        let browser = self.state.active_browser()?;
        browser
            .host()
            .with_context(|| format!("CEF browser {} has no host", browser.identifier()))
    }
}

impl Drop for CefRuntime {
    fn drop(&mut self) {
        if self.initialized {
            cef::shutdown();
            self.initialized = false;
        }
    }
}
