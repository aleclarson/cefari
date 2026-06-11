use std::path::{Path, PathBuf};

use cefari_core::{PackageFormat, RuntimePaths, packaged_resources_dir};

const CEF_RESOURCES_DIR_ENV: &str = "CEFARI_CEF_RESOURCES_DIR";

#[derive(Clone, Debug, Eq, PartialEq)]
struct CefRuntimePathConfig {
    cache_path: PathBuf,
    root_cache_path: PathBuf,
    log_file: PathBuf,
    browser_subprocess_path: PathBuf,
    resources_dir_path: Option<PathBuf>,
    locales_dir_path: Option<PathBuf>,
    framework_dir_path: Option<PathBuf>,
}

fn resolve_cef_runtime_paths(paths: &RuntimePaths) -> CefRuntimePathConfig {
    cef_runtime_path_config(
        paths,
        cef_resource_dir_candidates(paths),
        std::env::current_exe().unwrap_or_else(|_| PathBuf::from("cefari-desktop")),
    )
}

fn cef_runtime_path_config(
    paths: &RuntimePaths,
    resource_candidates: Vec<PathBuf>,
    browser_subprocess_path: PathBuf,
) -> CefRuntimePathConfig {
    let resources_dir_path = resource_candidates
        .into_iter()
        .find(|candidate| candidate.join("archive.json").is_file());
    let locales_dir_path = resources_dir_path
        .as_ref()
        .map(|resources_dir| resources_dir.join("locales"))
        .filter(|locales_dir| locales_dir.is_dir());
    let framework_dir_path = resources_dir_path
        .as_ref()
        .and_then(|resources_dir| cef_framework_dir(resources_dir));
    let root_cache_path = paths.cache_dir.join("cef");

    CefRuntimePathConfig {
        cache_path: root_cache_path.join("profile"),
        root_cache_path,
        log_file: paths.log_dir.join("cef.log"),
        browser_subprocess_path,
        resources_dir_path,
        locales_dir_path,
        framework_dir_path,
    }
}

fn cef_resource_dir_candidates(paths: &RuntimePaths) -> Vec<PathBuf> {
    let mut candidates = std::env::var_os(CEF_RESOURCES_DIR_ENV)
        .map(PathBuf::from)
        .into_iter()
        .collect::<Vec<_>>();

    candidates.extend(platform_package_formats().iter().filter_map(|format| {
        packaged_resources_dir(*format)
            .ok()
            .map(|resources_dir| resources_dir.join("cef"))
    }));
    candidates.push(paths.resource_dir.join("cef"));
    candidates
}

fn cef_framework_dir(resources_dir: &Path) -> Option<PathBuf> {
    let framework_dir = resources_dir.join("Chromium Embedded Framework.framework");
    framework_dir.is_dir().then_some(framework_dir)
}

#[cfg(target_os = "macos")]
fn platform_package_formats() -> &'static [PackageFormat] {
    &[PackageFormat::App, PackageFormat::Dmg]
}

#[cfg(target_os = "windows")]
fn platform_package_formats() -> &'static [PackageFormat] {
    &[PackageFormat::Nsis, PackageFormat::Wix]
}

#[cfg(all(unix, not(target_os = "macos")))]
fn platform_package_formats() -> &'static [PackageFormat] {
    &[
        PackageFormat::Deb,
        PackageFormat::AppImage,
        PackageFormat::Pacman,
    ]
}

#[cfg(not(any(target_os = "macos", target_os = "windows", unix)))]
fn platform_package_formats() -> &'static [PackageFormat] {
    &[]
}

mod imp {
    #![allow(clippy::transmute_ptr_to_ptr)]

    use std::{
        cell::RefCell,
        fs,
        path::PathBuf,
        ptr,
        rc::Rc,
        sync::{Arc, Mutex},
    };

    use anyhow::{Context, Result};
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use tao::window::Window;
    use tracing::{debug, error, info, warn};

    use cef::rc::Rc as _;
    use cef::wrapper::message_router::{
        BrowserSideCallback, BrowserSideHandler, BrowserSideRouter, MessageRouterBrowserSide,
        MessageRouterBrowserSideHandlerCallbacks, MessageRouterConfig, MessageRouterRendererSide,
        MessageRouterRendererSideHandlerCallbacks, RendererSideRouter,
    };
    use cef::wrapper::stream_resource_handler::StreamResourceHandler;
    use cef::{
        App, BrowserProcessHandler, Client, DownloadHandler, ImplApp, ImplBrowser as _,
        ImplBrowserHost as _, ImplBrowserProcessHandler, ImplClient, ImplDownloadHandler,
        ImplFrame as _, ImplLifeSpanHandler, ImplLoadHandler, ImplProcessMessage as _,
        ImplRenderHandler, ImplRenderProcessHandler, ImplRequest as _, ImplRequestHandler,
        ImplResourceRequestHandler, ImplSchemeRegistrar as _, LifeSpanHandler, LoadHandler,
        RenderHandler, RenderProcessHandler, RequestHandler, ResourceRequestHandler, SchemeOptions,
        WrapApp, WrapBrowserProcessHandler, WrapClient, WrapDownloadHandler, WrapLifeSpanHandler,
        WrapLoadHandler, WrapRenderHandler, WrapRenderProcessHandler, WrapRequestHandler,
        WrapResourceRequestHandler, wrap_app, wrap_browser_process_handler, wrap_client,
        wrap_download_handler, wrap_life_span_handler, wrap_load_handler, wrap_render_handler,
        wrap_render_process_handler, wrap_request_handler, wrap_resource_request_handler,
    };

    use crate::desktop_bridge::{
        BridgeOriginPolicy, NavigationDecision, NavigationPolicy, NavigationSurface,
        denied_response_json, origin_from_url, transport_error_response_json,
    };
    use crate::desktop_ui::{CEFARI_APP_SCHEME, diagnose_app_scheme_resource};
    use crate::external;

    use super::{CefRuntimePathConfig, resolve_cef_runtime_paths};

    pub struct CefRuntime {
        initialized: bool,
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

    #[allow(dead_code)]
    impl CefRuntime {
        pub fn initialize(paths: &cefari_core::RuntimePaths) -> Result<Self> {
            let args = cef::args::Args::new();
            let runtime_paths = resolve_cef_runtime_paths(paths);
            prepare_cef_runtime_dirs(&runtime_paths)?;
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
            let browser_router =
                <BrowserSideRouter as MessageRouterBrowserSide>::new(router_config);
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
            let size = window.inner_size();
            let bounds = cef::Rect {
                x: 0,
                y: 0,
                width: i32::try_from(size.width).unwrap_or(i32::MAX),
                height: i32::try_from(size.height).unwrap_or(i32::MAX),
            };
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

    fn cef_settings(runtime_paths: &CefRuntimePathConfig) -> cef::Settings {
        let mut settings = cef::Settings {
            no_sandbox: 1,
            external_message_pump: 1,
            browser_subprocess_path: cef_string_from_path(&runtime_paths.browser_subprocess_path),
            cache_path: cef_string_from_path(&runtime_paths.cache_path),
            root_cache_path: cef_string_from_path(&runtime_paths.root_cache_path),
            log_file: cef_string_from_path(&runtime_paths.log_file),
            ..Default::default()
        };

        if let Some(resources_dir_path) = &runtime_paths.resources_dir_path {
            settings.resources_dir_path = cef_string_from_path(resources_dir_path);
        }
        if let Some(locales_dir_path) = &runtime_paths.locales_dir_path {
            settings.locales_dir_path = cef_string_from_path(locales_dir_path);
        }
        if let Some(framework_dir_path) = &runtime_paths.framework_dir_path {
            settings.framework_dir_path = cef_string_from_path(framework_dir_path);
        }

        settings
    }

    fn cef_string_from_path(path: &std::path::Path) -> cef::CefString {
        cef::CefString::from(path.to_string_lossy().as_ref())
    }

    fn prepare_cef_runtime_dirs(runtime_paths: &CefRuntimePathConfig) -> Result<()> {
        fs::create_dir_all(&runtime_paths.cache_path).with_context(|| {
            format!(
                "failed to create CEF cache directory at {}",
                runtime_paths.cache_path.display()
            )
        })?;
        if let Some(log_dir) = runtime_paths.log_file.parent() {
            fs::create_dir_all(log_dir).with_context(|| {
                format!(
                    "failed to create CEF log directory at {}",
                    log_dir.display()
                )
            })?;
        }
        Ok(())
    }

    fn log_cef_runtime_paths(runtime_paths: &CefRuntimePathConfig) {
        info!(
            cache_path = %runtime_paths.cache_path.display(),
            root_cache_path = %runtime_paths.root_cache_path.display(),
            log_file = %runtime_paths.log_file.display(),
            browser_subprocess_path = %runtime_paths.browser_subprocess_path.display(),
            resources_dir_path = %display_optional_path(runtime_paths.resources_dir_path.as_ref()),
            locales_dir_path = %display_optional_path(runtime_paths.locales_dir_path.as_ref()),
            framework_dir_path = %display_optional_path(runtime_paths.framework_dir_path.as_ref()),
            "resolved CEF runtime paths"
        );
    }

    fn display_optional_path(path: Option<&PathBuf>) -> String {
        path.map_or_else(|| "<unset>".to_owned(), |path| path.display().to_string())
    }

    #[derive(Clone, Default)]
    struct SharedBrowserState(Rc<RefCell<BrowserState>>);

    #[allow(dead_code)]
    impl SharedBrowserState {
        fn has_browser(&self) -> bool {
            self.0.borrow().main_browser.is_some()
        }

        fn active_browser(&self) -> Result<cef::Browser> {
            self.0
                .borrow()
                .main_browser
                .clone()
                .context("CEF main browser is not available")
        }

        fn browser_created(&self, browser: &cef::Browser) {
            let identifier = browser.identifier();
            let is_popup = browser.is_popup() != 0;
            let mut state = self.0.borrow_mut();

            if state.main_browser.is_none() && !is_popup {
                state.main_browser = Some(browser.clone());
                info!(identifier, "CEF main browser retained");
            } else {
                debug!(
                    identifier,
                    is_popup,
                    has_main_browser = state.main_browser.is_some(),
                    "CEF browser created outside main-browser retention"
                );
            }
        }

        fn browser_closing(&self, browser: &cef::Browser) {
            let identifier = browser.identifier();
            let mut state = self.0.borrow_mut();
            let should_clear = state
                .main_browser
                .as_ref()
                .is_some_and(|main_browser| main_browser.identifier() == identifier);

            if should_clear {
                state.main_browser = None;
                info!(identifier, "CEF main browser released");
            } else {
                debug!(identifier, "CEF non-main browser closing");
            }
        }
    }

    #[derive(Default)]
    struct BrowserState {
        main_browser: Option<cef::Browser>,
    }

    #[derive(Clone, Default)]
    struct SharedBridgeIpcState(Arc<Mutex<BridgeIpcState>>);

    impl SharedBridgeIpcState {
        fn set_sender(&self, sender: Arc<dyn BridgeIpcSender>) {
            if let Ok(mut state) = self.0.lock() {
                state.sender = Some(sender);
            }
        }

        fn send(&self, request: CefBridgeIpcRequest) -> Result<()> {
            let sender = self
                .0
                .lock()
                .ok()
                .and_then(|state| state.sender.clone())
                .context("CEF bridge IPC sender is not installed")?;
            sender.send_bridge_ipc(request)
        }
    }

    #[derive(Default)]
    struct BridgeIpcState {
        sender: Option<Arc<dyn BridgeIpcSender>>,
    }

    #[derive(Clone, Default)]
    struct SharedMessagePumpState(Arc<Mutex<MessagePumpState>>);

    impl SharedMessagePumpState {
        fn set_scheduler(&self, scheduler: Arc<dyn MessagePumpScheduler>) {
            if let Ok(mut state) = self.0.lock() {
                state.scheduler = Some(scheduler);
            }
        }

        fn schedule(&self, delay_ms: i64) {
            let scheduler = self.0.lock().ok().and_then(|state| state.scheduler.clone());
            let Some(scheduler) = scheduler else {
                debug!(
                    delay_ms,
                    "CEF message pump work scheduled before Tao scheduler was installed"
                );
                return;
            };

            if let Err(error) = scheduler.schedule_message_pump_work(delay_ms) {
                warn!(%error, delay_ms, "failed to schedule CEF message pump work");
            }
        }
    }

    #[derive(Default)]
    struct MessagePumpState {
        scheduler: Option<Arc<dyn MessagePumpScheduler>>,
    }

    #[derive(Clone, Default)]
    struct SharedAppSchemeState(Arc<Mutex<AppSchemeState>>);

    impl SharedAppSchemeState {
        fn set_resource_dir(&self, resource_dir: PathBuf) {
            if let Ok(mut state) = self.0.lock() {
                state.resource_dir = Some(resource_dir);
            }
        }

        fn resource_handler_for_url(&self, url: &str) -> Result<cef::ResourceHandler, String> {
            let resource_dir = self
                .0
                .lock()
                .ok()
                .and_then(|state| state.resource_dir.clone())
                .ok_or_else(|| "app-scheme resource root is not installed".to_owned())?;
            let resource = diagnose_app_scheme_resource(&resource_dir, url)
                .map_err(|error| error.to_string())?;
            let path = resource.path.to_string_lossy();
            let stream =
                cef::stream_reader_create_for_file(Some(&cef::CefString::from(path.as_ref())))
                    .ok_or_else(|| {
                        format!(
                            "failed to open resource stream for {}",
                            resource.path.display()
                        )
                    })?;
            Ok(StreamResourceHandler::new_with_stream(
                resource.mime_type.to_owned(),
                stream,
            ))
        }
    }

    #[derive(Default)]
    struct AppSchemeState {
        resource_dir: Option<PathBuf>,
    }

    fn bridge_router_config() -> MessageRouterConfig {
        MessageRouterConfig {
            js_query_function: "__CEFARI_IPC_QUERY__".to_owned(),
            js_cancel_function: "__CEFARI_IPC_QUERY_CANCEL__".to_owned(),
            ..Default::default()
        }
    }

    wrap_app! {
        struct CefariApp {
            browser_process_handler: cef::BrowserProcessHandler,
            render_process_handler: cef::RenderProcessHandler,
        }

        impl App {
            fn browser_process_handler(&self) -> Option<cef::BrowserProcessHandler> {
                Some(self.browser_process_handler.clone())
            }

            fn on_register_custom_schemes(
                &self,
                registrar: Option<&mut cef::SchemeRegistrar>,
            ) {
                register_app_scheme(registrar);
            }

            fn render_process_handler(&self) -> Option<cef::RenderProcessHandler> {
                Some(self.render_process_handler.clone())
            }
        }
    }

    impl CefariApp {
        fn build(
            router_config: MessageRouterConfig,
            message_pump: SharedMessagePumpState,
        ) -> cef::App {
            Self::new(
                CefariBrowserProcessHandler::new(message_pump),
                CefariRenderProcessHandler::build(router_config),
            )
        }
    }

    wrap_browser_process_handler! {
        struct CefariBrowserProcessHandler {
            message_pump: SharedMessagePumpState,
        }

        impl BrowserProcessHandler {
            fn on_schedule_message_pump_work(&self, delay_ms: i64) {
                self.message_pump.schedule(delay_ms);
            }
        }
    }

    fn register_app_scheme(registrar: Option<&mut cef::SchemeRegistrar>) {
        let Some(registrar) = registrar else {
            warn!("CEF app scheme registrar was unavailable");
            return;
        };
        let scheme = cef::CefString::from(CEFARI_APP_SCHEME);
        let options = SchemeOptions::STANDARD.get_raw()
            | SchemeOptions::LOCAL.get_raw()
            | SchemeOptions::SECURE.get_raw()
            | SchemeOptions::CORS_ENABLED.get_raw()
            | SchemeOptions::FETCH_ENABLED.get_raw();
        let options =
            ::std::os::raw::c_int::try_from(options).expect("CEF scheme options should fit c_int");
        let registered = registrar.add_custom_scheme(Some(&scheme), options);
        if registered == 1 {
            info!(scheme = CEFARI_APP_SCHEME, "registered CEF app scheme");
        } else {
            warn!(
                scheme = CEFARI_APP_SCHEME,
                result = registered,
                "failed to register CEF app scheme"
            );
        }
    }

    wrap_render_process_handler! {
        struct CefariRenderProcessHandler {
            router: Arc<RendererSideRouter>,
        }

        impl RenderProcessHandler {
            fn on_context_created(
                &self,
                browser: Option<&mut cef::Browser>,
                frame: Option<&mut cef::Frame>,
                context: Option<&mut cef::V8Context>,
            ) {
                self.router.on_context_created(
                    browser.as_deref().cloned(),
                    frame.as_deref().cloned(),
                    context.as_deref().cloned(),
                );
                debug!(frame_url = %frame_url(frame), "CEF render context created");
            }

            fn on_context_released(
                &self,
                browser: Option<&mut cef::Browser>,
                frame: Option<&mut cef::Frame>,
                context: Option<&mut cef::V8Context>,
            ) {
                self.router.on_context_released(
                    browser.as_deref().cloned(),
                    frame.as_deref().cloned(),
                    context.as_deref().cloned(),
                );
                debug!(frame_url = %frame_url(frame), "CEF render context released");
            }

            fn on_process_message_received(
                &self,
                browser: Option<&mut cef::Browser>,
                frame: Option<&mut cef::Frame>,
                source_process: cef::ProcessId,
                message: Option<&mut cef::ProcessMessage>,
            ) -> ::std::os::raw::c_int {
                i32::from(self.router.on_process_message_received(
                    browser.as_deref().cloned(),
                    frame.as_deref().cloned(),
                    Some(source_process),
                    message.as_deref().cloned(),
                ))
            }
        }
    }

    impl CefariRenderProcessHandler {
        fn build(router_config: MessageRouterConfig) -> cef::RenderProcessHandler {
            Self::new(<RendererSideRouter as MessageRouterRendererSide>::new(
                router_config,
            ))
        }
    }

    wrap_client! {
        struct CefariCefClient {
            browser_router: Arc<BrowserSideRouter>,
            download_handler: cef::DownloadHandler,
            life_span_handler: cef::LifeSpanHandler,
            load_handler: cef::LoadHandler,
            render_handler: cef::RenderHandler,
            request_handler: cef::RequestHandler,
        }

        impl Client {
            fn life_span_handler(&self) -> Option<cef::LifeSpanHandler> {
                Some(self.life_span_handler.clone())
            }

            fn load_handler(&self) -> Option<cef::LoadHandler> {
                Some(self.load_handler.clone())
            }

            fn render_handler(&self) -> Option<cef::RenderHandler> {
                Some(self.render_handler.clone())
            }

            fn request_handler(&self) -> Option<cef::RequestHandler> {
                Some(self.request_handler.clone())
            }

            fn download_handler(&self) -> Option<cef::DownloadHandler> {
                Some(self.download_handler.clone())
            }

            fn on_process_message_received(
                &self,
                browser: Option<&mut cef::Browser>,
                frame: Option<&mut cef::Frame>,
                source_process: cef::ProcessId,
                message: Option<&mut cef::ProcessMessage>,
            ) -> ::std::os::raw::c_int {
                if self.browser_router.on_process_message_received(
                    browser.as_deref().cloned(),
                    frame.as_deref().cloned(),
                    source_process,
                    message.as_deref().cloned(),
                ) {
                    debug!(message = %message_name(message), "CEF process message handled by bridge router");
                    return 1;
                }

                debug!(
                    frame_url = %frame_url(frame),
                    source_process = ?source_process,
                    message = %message_name(message),
                    "CEF process message received"
                );
                0
            }
        }
    }

    impl CefariCefClient {
        fn build(
            state: SharedBrowserState,
            origin_policy: BridgeOriginPolicy,
            navigation_policy: NavigationPolicy,
            browser_router: Arc<BrowserSideRouter>,
            app_scheme: SharedAppSchemeState,
        ) -> cef::Client {
            Self::new(
                browser_router.clone(),
                CefariDownloadHandler::new(navigation_policy.clone()),
                CefariLifeSpanHandler::new(
                    state,
                    browser_router.clone(),
                    navigation_policy.clone(),
                ),
                CefariLoadHandler::new(),
                CefariRenderHandler::new(),
                CefariRequestHandler::new(
                    origin_policy,
                    navigation_policy,
                    browser_router,
                    app_scheme,
                ),
            )
        }
    }

    wrap_download_handler! {
        struct CefariDownloadHandler {
            navigation_policy: NavigationPolicy,
        }

        impl DownloadHandler {
            fn can_download(
                &self,
                _browser: Option<&mut cef::Browser>,
                url: Option<&cef::CefString>,
                request_method: Option<&cef::CefString>,
            ) -> ::std::os::raw::c_int {
                let url = optional_cef_string(url);
                let decision = self
                    .navigation_policy
                    .decide(NavigationSurface::Download, &url);
                warn!(
                    url,
                    request_method = %optional_cef_string(request_method),
                    reason = decision.reason,
                    "denied CEF download"
                );
                0
            }

            fn on_before_download(
                &self,
                _browser: Option<&mut cef::Browser>,
                _download_item: Option<&mut cef::DownloadItem>,
                suggested_name: Option<&cef::CefString>,
                _callback: Option<&mut cef::BeforeDownloadCallback>,
            ) -> ::std::os::raw::c_int {
                warn!(
                    suggested_name = %optional_cef_string(suggested_name),
                    "denied CEF download before start"
                );
                1
            }
        }
    }

    wrap_life_span_handler! {
        struct CefariLifeSpanHandler {
            state: SharedBrowserState,
            browser_router: Arc<BrowserSideRouter>,
            navigation_policy: NavigationPolicy,
        }

        impl LifeSpanHandler {
            fn on_before_popup(
                &self,
                _browser: Option<&mut cef::Browser>,
                _frame: Option<&mut cef::Frame>,
                popup_id: ::std::os::raw::c_int,
                target_url: Option<&cef::CefString>,
                target_frame_name: Option<&cef::CefString>,
                target_disposition: cef::WindowOpenDisposition,
                user_gesture: ::std::os::raw::c_int,
                _popup_features: Option<&cef::PopupFeatures>,
                _window_info: Option<&mut cef::WindowInfo>,
                _client: Option<&mut Option<cef::Client>>,
                _settings: Option<&mut cef::BrowserSettings>,
                _extra_info: Option<&mut Option<cef::DictionaryValue>>,
                no_javascript_access: Option<&mut ::std::os::raw::c_int>,
            ) -> ::std::os::raw::c_int {
                if let Some(no_javascript_access) = no_javascript_access {
                    *no_javascript_access = 1;
                }
                let target_url = optional_cef_string(target_url);
                handle_navigation_decision(
                    &self.navigation_policy,
                    NavigationSurface::Popup,
                    &target_url,
                    "CEF popup",
                    &[
                        ("popup_id", popup_id.to_string()),
                        ("target_frame", optional_cef_string(target_frame_name)),
                        ("disposition", format!("{target_disposition:?}")),
                        ("user_gesture", (user_gesture != 0).to_string()),
                    ],
                )
            }

            fn on_after_created(&self, browser: Option<&mut cef::Browser>) {
                if let Some(browser) = browser {
                    self.state.browser_created(browser);
                }
                info!("CEF browser lifecycle created");
            }

            fn do_close(&self, _browser: Option<&mut cef::Browser>) -> ::std::os::raw::c_int {
                debug!("CEF browser lifecycle close requested");
                0
            }

            fn on_before_close(&self, browser: Option<&mut cef::Browser>) {
                self.browser_router.on_before_close(browser.as_deref().cloned());
                if let Some(browser) = browser {
                    self.state.browser_closing(browser);
                }
                info!("CEF browser lifecycle closing");
            }
        }
    }

    wrap_load_handler! {
        struct CefariLoadHandler;

        impl LoadHandler {
            fn on_loading_state_change(
                &self,
                _browser: Option<&mut cef::Browser>,
                is_loading: ::std::os::raw::c_int,
                can_go_back: ::std::os::raw::c_int,
                can_go_forward: ::std::os::raw::c_int,
            ) {
                debug!(
                    is_loading = is_loading != 0,
                    can_go_back = can_go_back != 0,
                    can_go_forward = can_go_forward != 0,
                    "CEF page loading state changed"
                );
            }

            fn on_load_start(
                &self,
                _browser: Option<&mut cef::Browser>,
                frame: Option<&mut cef::Frame>,
                transition_type: cef::TransitionType,
            ) {
                debug!(
                    frame_url = %frame_url(frame),
                    transition_type = ?transition_type,
                    "CEF frame load started"
                );
            }

            fn on_load_end(
                &self,
                _browser: Option<&mut cef::Browser>,
                frame: Option<&mut cef::Frame>,
                http_status_code: ::std::os::raw::c_int,
            ) {
                info!(
                    frame_url = %frame_url(frame),
                    http_status_code,
                    "CEF frame load completed"
                );
            }

            fn on_load_error(
                &self,
                _browser: Option<&mut cef::Browser>,
                frame: Option<&mut cef::Frame>,
                error_code: cef::Errorcode,
                error_text: Option<&cef::CefString>,
                failed_url: Option<&cef::CefString>,
            ) {
                error!(
                    frame_url = %frame_url(frame),
                    error_code = ?error_code,
                    error_text = %optional_cef_string(error_text),
                    failed_url = %optional_cef_string(failed_url),
                    "CEF frame load failed"
                );
            }
        }
    }

    wrap_request_handler! {
        struct CefariRequestHandler {
            origin_policy: BridgeOriginPolicy,
            navigation_policy: NavigationPolicy,
            browser_router: Arc<BrowserSideRouter>,
            app_scheme: SharedAppSchemeState,
        }

        impl RequestHandler {
            fn resource_request_handler(
                &self,
                _browser: Option<&mut cef::Browser>,
                _frame: Option<&mut cef::Frame>,
                request: Option<&mut cef::Request>,
                is_navigation: ::std::os::raw::c_int,
                is_download: ::std::os::raw::c_int,
                request_initiator: Option<&cef::CefString>,
                disable_default_handling: Option<&mut ::std::os::raw::c_int>,
            ) -> Option<ResourceRequestHandler> {
                let url = request_url(request);
                if !url.starts_with("cefari://") {
                    return None;
                }

                if let Some(disable_default_handling) = disable_default_handling {
                    *disable_default_handling = 1;
                }
                debug!(
                    url,
                    is_navigation = is_navigation != 0,
                    is_download = is_download != 0,
                    request_initiator = %optional_cef_string(request_initiator),
                    "handling CEF app-scheme resource request"
                );
                Some(CefariAppResourceRequestHandler::new(
                    self.app_scheme.clone(),
                ))
            }

            fn on_before_browse(
                &self,
                browser: Option<&mut cef::Browser>,
                frame: Option<&mut cef::Frame>,
                request: Option<&mut cef::Request>,
                user_gesture: ::std::os::raw::c_int,
                is_redirect: ::std::os::raw::c_int,
            ) -> ::std::os::raw::c_int {
                let is_main_frame = frame
                    .as_ref()
                    .is_some_and(|frame| frame.is_main() != 0);
                let current_frame_url = frame
                    .as_ref()
                    .map(|frame| cef::CefString::from(&frame.url()).to_string())
                    .unwrap_or_default();
                let target_url = request_url(request);
                self.browser_router.on_before_browse(
                    browser.as_deref().cloned(),
                    frame.as_deref().cloned(),
                );
                let surface = if is_main_frame {
                    NavigationSurface::MainFrame
                } else {
                    NavigationSurface::SubFrame
                };
                let policy_result = handle_navigation_decision(
                    &self.navigation_policy,
                    surface,
                    &target_url,
                    "CEF navigation",
                    &[
                        ("frame_url", current_frame_url.clone()),
                        ("user_gesture", (user_gesture != 0).to_string()),
                        ("is_redirect", (is_redirect != 0).to_string()),
                    ],
                );
                if policy_result != 0 {
                    return policy_result;
                }
                debug!(
                    frame_url = %current_frame_url,
                    request_url = %target_url,
                    user_gesture = user_gesture != 0,
                    is_redirect = is_redirect != 0,
                    "CEF navigation requested"
                );
                0
            }

            fn on_open_urlfrom_tab(
                &self,
                _browser: Option<&mut cef::Browser>,
                frame: Option<&mut cef::Frame>,
                target_url: Option<&cef::CefString>,
                target_disposition: cef::WindowOpenDisposition,
                user_gesture: ::std::os::raw::c_int,
            ) -> ::std::os::raw::c_int {
                let target_url = optional_cef_string(target_url);
                handle_navigation_decision(
                    &self.navigation_policy,
                    NavigationSurface::OpenUrlFromTab,
                    &target_url,
                    "CEF open-url-from-tab",
                    &[
                        ("frame_url", frame_url(frame)),
                        ("disposition", format!("{target_disposition:?}")),
                        ("user_gesture", (user_gesture != 0).to_string()),
                    ],
                )
            }

            fn on_render_view_ready(&self, _browser: Option<&mut cef::Browser>) {
                debug!("CEF render view ready");
            }

            fn on_render_process_unresponsive(
                &self,
                _browser: Option<&mut cef::Browser>,
                _callback: Option<&mut cef::UnresponsiveProcessCallback>,
            ) -> ::std::os::raw::c_int {
                warn!("CEF render process unresponsive");
                0
            }

            fn on_render_process_responsive(&self, _browser: Option<&mut cef::Browser>) {
                info!("CEF render process responsive");
            }

            fn on_render_process_terminated(
                &self,
                browser: Option<&mut cef::Browser>,
                status: cef::TerminationStatus,
                error_code: ::std::os::raw::c_int,
                error_string: Option<&cef::CefString>,
            ) {
                self.browser_router
                    .on_render_process_terminated(browser.as_deref().cloned());
                error!(
                    status = ?status,
                    error_code,
                    error_string = %optional_cef_string(error_string),
                    "CEF render process terminated"
                );
            }

            fn on_document_available_in_main_frame(&self, browser: Option<&mut cef::Browser>) {
                if let Some(mut frame) = browser.and_then(|browser| browser.main_frame()) {
                    inject_bridge_script(&self.origin_policy, &mut frame);
                }
                debug!("CEF document available in main frame");
            }
        }
    }

    wrap_resource_request_handler! {
        struct CefariAppResourceRequestHandler {
            app_scheme: SharedAppSchemeState,
        }

        impl ResourceRequestHandler {
            fn resource_handler(
                &self,
                _browser: Option<&mut cef::Browser>,
                _frame: Option<&mut cef::Frame>,
                request: Option<&mut cef::Request>,
            ) -> Option<cef::ResourceHandler> {
                let url = request_url(request);
                match self.app_scheme.resource_handler_for_url(&url) {
                    Ok(handler) => Some(handler),
                    Err(reason) => {
                        warn!(url, reason, "CEF app-scheme resource request failed");
                        None
                    }
                }
            }
        }
    }

    struct CefariBridgeIpcHandler {
        bridge_ipc: SharedBridgeIpcState,
        origin_policy: BridgeOriginPolicy,
    }

    impl CefariBridgeIpcHandler {
        fn new(bridge_ipc: SharedBridgeIpcState, origin_policy: BridgeOriginPolicy) -> Self {
            Self {
                bridge_ipc,
                origin_policy,
            }
        }
    }

    impl BrowserSideHandler for CefariBridgeIpcHandler {
        fn on_query_str(
            &self,
            _browser: Option<cef::Browser>,
            frame: Option<cef::Frame>,
            _query_id: i64,
            request: &str,
            _persistent: bool,
            callback: Arc<Mutex<dyn BrowserSideCallback>>,
        ) -> bool {
            let frame_url = frame
                .as_ref()
                .map(|frame| cef::CefString::from(&frame.url()).to_string())
                .unwrap_or_default();
            let origin = origin_from_url(&frame_url).unwrap_or_default();
            if !self.origin_policy.is_trusted_origin(&origin) {
                let response =
                    denied_response_json(request, "origin is not allowed to use the Cefari bridge");
                if let Ok(callback) = callback.lock() {
                    callback.success_str(&response);
                }
                warn!(
                    frame_url,
                    origin, "denied CEF bridge IPC from untrusted frame"
                );
                return true;
            }

            if let Err(error) = self.bridge_ipc.send(CefBridgeIpcRequest {
                origin,
                request_json: request.to_owned(),
                callback: callback.clone(),
            }) {
                if let Ok(callback) = callback.lock() {
                    let response =
                        transport_error_response_json(request, "native IPC transport failed");
                    callback.success_str(&response);
                }
                error!(
                    %error,
                    frame_url,
                    "failed to enqueue CEF bridge IPC request"
                );
            }

            true
        }
    }

    wrap_render_handler! {
        struct CefariRenderHandler;

        impl RenderHandler {
            fn on_virtual_keyboard_requested(
                &self,
                _browser: Option<&mut cef::Browser>,
                input_mode: cef::TextInputMode,
            ) {
                debug!(input_mode = ?input_mode, "CEF virtual keyboard requested");
            }
        }
    }

    fn optional_cef_string(value: Option<&cef::CefString>) -> String {
        value.map(ToString::to_string).unwrap_or_default()
    }

    fn frame_url(frame: Option<&mut cef::Frame>) -> String {
        frame
            .map(|frame| cef::CefString::from(&frame.url()).to_string())
            .unwrap_or_default()
    }

    fn request_url(request: Option<&mut cef::Request>) -> String {
        request
            .map(|request| cef::CefString::from(&request.url()).to_string())
            .unwrap_or_default()
    }

    fn message_name(message: Option<&mut cef::ProcessMessage>) -> String {
        message
            .map(|message| cef::CefString::from(&message.name()).to_string())
            .unwrap_or_default()
    }

    fn handle_navigation_decision(
        policy: &NavigationPolicy,
        surface: NavigationSurface,
        url: &str,
        operation: &str,
        details: &[(&str, String)],
    ) -> ::std::os::raw::c_int {
        let decision = policy.decide(surface, url);
        match decision.decision {
            NavigationDecision::Allow => {
                debug!(url, surface = ?surface, reason = decision.reason, operation, "allowed CEF navigation");
                0
            }
            NavigationDecision::OpenExternally => {
                match external::open_external_url(url) {
                    Ok(()) => {
                        info!(
                            url,
                            surface = ?surface,
                            reason = decision.reason,
                            operation,
                            details = ?details,
                            "opened CEF navigation externally"
                        );
                    }
                    Err(error) => {
                        warn!(
                            url,
                            surface = ?surface,
                            reason = decision.reason,
                            operation,
                            details = ?details,
                            %error,
                            "failed to open CEF navigation externally"
                        );
                    }
                }
                1
            }
            NavigationDecision::Deny => {
                warn!(
                    url,
                    surface = ?surface,
                    reason = decision.reason,
                    operation,
                    details = ?details,
                    "denied CEF navigation"
                );
                1
            }
        }
    }

    fn inject_bridge_script(origin_policy: &BridgeOriginPolicy, frame: &mut cef::Frame) {
        if frame.is_main() == 0 {
            debug!("skipping Cefari bridge injection for non-main frame");
            return;
        }

        let frame_url = cef::CefString::from(&frame.url()).to_string();
        let origin = origin_from_url(&frame_url);
        let Some(script) = origin_policy.bridge_script_for_url(&frame_url) else {
            debug!(
                frame_url,
                origin = origin.as_deref().unwrap_or_default(),
                "skipping Cefari bridge injection for untrusted frame"
            );
            return;
        };

        let code = cef::CefString::from(script);
        let script_url = cef::CefString::from("cefari://bridge/bootstrap.js");
        frame.execute_java_script(Some(&code), Some(&script_url), 1);
        info!(
            frame_url,
            origin = origin.as_deref().unwrap_or_default(),
            "Cefari bridge script injected"
        );
    }

    impl Drop for CefRuntime {
        fn drop(&mut self) {
            if self.initialized {
                cef::shutdown();
                self.initialized = false;
                info!("CEF shut down");
            }
        }
    }

    pub fn initialize(paths: &cefari_core::RuntimePaths) -> Result<CefRuntime> {
        CefRuntime::initialize(paths).context("failed to initialize CEF")
    }

    fn native_window_handle(window: &Window) -> Result<cef::sys::cef_window_handle_t> {
        match window
            .window_handle()
            .context("failed to get native window handle")?
            .as_raw()
        {
            #[cfg(target_os = "macos")]
            RawWindowHandle::AppKit(handle) => Ok(handle.ns_view.as_ptr().cast()),
            #[cfg(target_os = "windows")]
            RawWindowHandle::Win32(handle) => {
                Ok(handle.hwnd.get() as cef::sys::cef_window_handle_t)
            }
            #[cfg(all(unix, not(target_os = "macos")))]
            RawWindowHandle::Xlib(handle) => Ok(handle.window as cef::sys::cef_window_handle_t),
            other => anyhow::bail!("unsupported native window handle for CEF: {other:?}"),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::SharedBrowserState;

        #[test]
        fn empty_browser_state_reports_missing_browser() {
            let state = SharedBrowserState::default();

            assert!(!state.has_browser());
            let Err(error) = state.active_browser() else {
                panic!("empty state should not return a browser");
            };

            assert!(
                error
                    .to_string()
                    .contains("CEF main browser is not available")
            );
        }
    }
}

pub use imp::{BridgeIpcSender, CefBridgeIpcRequest, CefRuntime, MessagePumpScheduler};

pub fn initialize(paths: &cefari_core::RuntimePaths) -> anyhow::Result<CefRuntime> {
    let runtime = imp::initialize(paths)?;
    tracing::info!("CEF runtime prepared");
    Ok(runtime)
}

#[cfg(test)]
mod path_tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use cefari_core::RuntimePaths;

    use super::cef_runtime_path_config;

    #[test]
    fn derives_runtime_cache_log_and_subprocess_paths() {
        let root = temp_dir("runtime-paths");
        let paths = test_paths(&root);
        let subprocess = root.join("bin/cefari-desktop");

        let config = cef_runtime_path_config(&paths, Vec::new(), subprocess.clone());

        assert_eq!(config.cache_path, root.join("cache/cef/profile"));
        assert_eq!(config.root_cache_path, root.join("cache/cef"));
        assert_eq!(config.log_file, root.join("data/logs/cef.log"));
        assert_eq!(config.browser_subprocess_path, subprocess);
        assert!(config.resources_dir_path.is_none());
        assert!(config.locales_dir_path.is_none());
        assert!(config.framework_dir_path.is_none());

        if root.exists() {
            fs::remove_dir_all(root).expect("temp dir should be removable");
        }
    }

    #[test]
    fn selects_first_valid_cef_resource_candidate() {
        let root = temp_dir("resource-candidates");
        let missing = root.join("missing-cef");
        let resources = root.join("resources-cef");
        fs::create_dir_all(resources.join("locales")).expect("locales dir should exist");
        fs::create_dir_all(resources.join("Chromium Embedded Framework.framework"))
            .expect("framework dir should exist");
        fs::write(resources.join("archive.json"), "{}").expect("archive metadata should exist");
        let paths = test_paths(&root);

        let config = cef_runtime_path_config(
            &paths,
            vec![missing, resources.clone()],
            root.join("cefari-desktop"),
        );

        assert_eq!(config.resources_dir_path, Some(resources.clone()));
        assert_eq!(config.locales_dir_path, Some(resources.join("locales")));
        assert_eq!(
            config.framework_dir_path,
            Some(resources.join("Chromium Embedded Framework.framework"))
        );

        fs::remove_dir_all(root).expect("temp dir should be removable");
    }

    #[test]
    fn resolves_runtime_resource_dir_as_fallback_candidate() {
        let root = temp_dir("runtime-fallback");
        let paths = test_paths(&root);
        let cef_dir = paths.resource_dir.join("cef");
        fs::create_dir_all(&cef_dir).expect("CEF resource dir should exist");
        fs::write(cef_dir.join("archive.json"), "{}").expect("archive metadata should exist");

        let config = cef_runtime_path_config(&paths, vec![cef_dir.clone()], root.join("desktop"));

        assert_eq!(config.resources_dir_path, Some(cef_dir));

        fs::remove_dir_all(root).expect("temp dir should be removable");
    }

    fn test_paths(root: &Path) -> RuntimePaths {
        RuntimePaths {
            config_dir: root.join("config"),
            config_file: root.join("config/cefari.json"),
            data_dir: root.join("data"),
            cache_dir: root.join("cache"),
            log_dir: root.join("data/logs"),
            resource_dir: root.join("data/resources"),
            update_dir: root.join("data/updates"),
        }
    }

    fn temp_dir(label: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("cefari-desktop-cef-{label}-{suffix}"))
    }
}
