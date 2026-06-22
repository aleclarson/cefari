#[cfg(target_os = "macos")]
mod macos_helpers;
mod paths;

const CEFARI_SMOKE_BACKGROUND_ENV: &str = "CEFARI_SMOKE_BACKGROUND";

mod imp {
    #![allow(clippy::transmute_ptr_to_ptr)]

    mod bridge;
    mod navigation;
    mod runtime;
    mod state;

    use std::{fs, path::PathBuf, sync::Arc};

    use anyhow::{Context, Result};
    use cefari_core::{BrowserConfig, CefariIpcEvent};
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use tao::dpi::PhysicalSize;
    use tao::window::Window;
    use tracing::{debug, error, info, warn};

    use cef::rc::Rc as _;
    use cef::wrapper::message_router::{
        BrowserSideRouter, MessageRouterBrowserSideHandlerCallbacks, MessageRouterConfig,
        MessageRouterRendererSide, MessageRouterRendererSideHandlerCallbacks, RendererSideRouter,
    };
    use cef::{
        App, BrowserProcessHandler, Client, DownloadHandler, ImplApp, ImplBeforeDownloadCallback,
        ImplBrowser as _, ImplBrowserProcessHandler, ImplClient, ImplCommandLine as _,
        ImplDownloadHandler, ImplDownloadItem as _, ImplFrame as _, ImplLifeSpanHandler,
        ImplLoadHandler, ImplRenderHandler, ImplRenderProcessHandler, ImplRequestHandler,
        ImplResourceRequestHandler, ImplSchemeRegistrar as _, LifeSpanHandler, LoadHandler,
        RenderHandler, RenderProcessHandler, RequestHandler, ResourceRequestHandler, SchemeOptions,
        WrapApp, WrapBrowserProcessHandler, WrapClient, WrapDownloadHandler, WrapLifeSpanHandler,
        WrapLoadHandler, WrapRenderHandler, WrapRenderProcessHandler, WrapRequestHandler,
        WrapResourceRequestHandler, wrap_app, wrap_browser_process_handler, wrap_client,
        wrap_download_handler, wrap_life_span_handler, wrap_load_handler, wrap_render_handler,
        wrap_render_process_handler, wrap_request_handler, wrap_resource_request_handler,
    };

    use crate::desktop_bridge::{BridgeOriginPolicy, NavigationPolicy, NavigationSurface};
    use crate::desktop_downloads::{
        DownloadDecision, DownloadPolicy, DownloadSnapshot, SharedDownloadState,
    };
    use crate::desktop_ui::CEFARI_APP_SCHEME;

    use super::paths::CefRuntimePathConfig;
    use bridge::{CefariBridgeIpcHandler, bridge_router_config};
    use navigation::{
        cef_userfree_string, frame_url, handle_navigation_decision, inject_bridge_script,
        message_name, optional_cef_string, request_url,
    };
    pub use runtime::{BridgeIpcSender, CefBridgeIpcRequest, CefRuntime, MessagePumpScheduler};
    use state::{SharedAppSchemeState, SharedBrowserState, SharedMessagePumpState};

    fn cef_settings(
        runtime_paths: &CefRuntimePathConfig,
        devtools_endpoint: Option<crate::desktop_devtools::DevtoolsEndpoint>,
    ) -> cef::Settings {
        let mut settings = cef::Settings {
            no_sandbox: 1,
            external_message_pump: 1,
            cache_path: cef_string_from_path(&runtime_paths.cache_path),
            root_cache_path: cef_string_from_path(&runtime_paths.root_cache_path),
            log_file: cef_string_from_path(&runtime_paths.log_file),
            ..Default::default()
        };

        if let Some(browser_subprocess_path) = &runtime_paths.browser_subprocess_path {
            settings.browser_subprocess_path = cef_string_from_path(browser_subprocess_path);
        }
        if let Some(main_bundle_path) = &runtime_paths.main_bundle_path {
            settings.main_bundle_path = cef_string_from_path(main_bundle_path);
        }
        if let Some(resources_dir_path) = &runtime_paths.resources_dir_path {
            settings.resources_dir_path = cef_string_from_path(resources_dir_path);
        }
        if let Some(locales_dir_path) = &runtime_paths.locales_dir_path {
            settings.locales_dir_path = cef_string_from_path(locales_dir_path);
        }
        if let Some(framework_dir_path) = &runtime_paths.framework_dir_path {
            settings.framework_dir_path = cef_string_from_path(framework_dir_path);
        }
        if let Some(endpoint) = devtools_endpoint {
            settings.remote_debugging_port = i32::from(endpoint.port.get());
            info!(
                port = endpoint.port.get(),
                "enabled CEF Chrome DevTools Protocol remote debugging"
            );
        }

        settings
    }

    fn configure_cef_api_version() -> Result<()> {
        let hash = cef::api_hash(cef::sys::CEF_API_VERSION_LAST, 0);
        if hash.is_null() {
            anyhow::bail!(
                "failed to configure CEF API version {}",
                cef::sys::CEF_API_VERSION_LAST
            );
        }

        info!(version = cef::api_version(), "configured CEF API version");
        Ok(())
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
            executable_path = %runtime_paths.executable_path.display(),
            browser_subprocess_path = %display_optional_path(runtime_paths.browser_subprocess_path.as_ref()),
            main_bundle_path = %display_optional_path(runtime_paths.main_bundle_path.as_ref()),
            resources_dir_path = %display_optional_path(runtime_paths.resources_dir_path.as_ref()),
            locales_dir_path = %display_optional_path(runtime_paths.locales_dir_path.as_ref()),
            framework_dir_path = %display_optional_path(runtime_paths.framework_dir_path.as_ref()),
            "resolved CEF runtime paths"
        );
    }

    fn display_optional_path(path: Option<&PathBuf>) -> String {
        path.map_or_else(|| "<unset>".to_owned(), |path| path.display().to_string())
    }

    wrap_app! {
        struct CefariApp {
            browser_config: BrowserConfig,
            browser_process_handler: cef::BrowserProcessHandler,
            render_process_handler: cef::RenderProcessHandler,
        }

        impl App {
            fn on_before_command_line_processing(
                &self,
                process_type: Option<&cef::CefString>,
                command_line: Option<&mut cef::CommandLine>,
            ) {
                configure_chromium_command_line(&self.browser_config, process_type, command_line);
            }

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

    fn configure_chromium_command_line(
        browser_config: &BrowserConfig,
        process_type: Option<&cef::CefString>,
        command_line: Option<&mut cef::CommandLine>,
    ) {
        let Some(command_line) = command_line else {
            return;
        };

        if browser_config.webgpu {
            append_webgpu_chromium_switches(command_line);
        }

        if development_chromium_switches_requested() {
            append_chromium_switch(command_line, "use-mock-keychain");
            append_chromium_switch_with_value(command_line, "password-store", "basic");
            append_chromium_switch(command_line, "disable-save-password-bubble");
            append_chromium_switch(command_line, "disable-notifications");
            append_chromium_switch(command_line, "deny-permission-prompts");
        }

        if smoke_background_chromium_switches_requested() {
            append_chromium_switch(command_line, "disable-gpu");
            append_chromium_switch(command_line, "disable-gpu-compositing");
            append_chromium_switch(command_line, "disable-gpu-sandbox");
        }

        debug!(
            process_type = %display_cef_process_type(process_type),
            webgpu = browser_config.webgpu,
            "configured Chromium command line"
        );
    }

    fn append_webgpu_chromium_switches(command_line: &cef::CommandLine) {
        append_chromium_switch(command_line, "enable-unsafe-webgpu");
        #[cfg(target_os = "linux")]
        append_chromium_feature(command_line, "Vulkan");
    }

    fn development_chromium_switches_requested() -> bool {
        development_chromium_switches_requested_from(
            crate::desktop_devtools::dev_mode_enabled(),
            std::env::var(super::CEFARI_SMOKE_BACKGROUND_ENV).as_deref() == Ok("1"),
        )
    }

    fn smoke_background_chromium_switches_requested() -> bool {
        std::env::var(super::CEFARI_SMOKE_BACKGROUND_ENV).as_deref() == Ok("1")
    }

    fn development_chromium_switches_requested_from(
        dev_mode: bool,
        smoke_background: bool,
    ) -> bool {
        dev_mode || smoke_background
    }

    fn append_chromium_switch(command_line: &cef::CommandLine, name: &str) {
        let name = cef::CefString::from(name);
        command_line.append_switch(Some(&name));
    }

    fn append_chromium_switch_with_value(command_line: &cef::CommandLine, name: &str, value: &str) {
        let name = cef::CefString::from(name);
        let value = cef::CefString::from(value);
        command_line.append_switch_with_value(Some(&name), Some(&value));
    }

    #[cfg(target_os = "linux")]
    fn append_chromium_feature(command_line: &cef::CommandLine, feature: &str) {
        let name = cef::CefString::from("enable-features");
        let existing = cef_userfree_string(&command_line.switch_value(Some(&name)));
        let value = append_chromium_feature_value(&existing, feature);
        let value = cef::CefString::from(value.as_str());
        command_line.append_switch_with_value(Some(&name), Some(&value));
    }

    #[cfg(any(test, target_os = "linux"))]
    fn append_chromium_feature_value(existing: &str, feature: &str) -> String {
        let mut features = existing
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>();
        if !features.contains(&feature) {
            features.push(feature);
        }
        features.join(",")
    }

    fn display_cef_process_type(process_type: Option<&cef::CefString>) -> String {
        process_type
            .and_then(cef::CefString::as_slice)
            .map(String::from_utf16_lossy)
            .filter(|process_type| !process_type.is_empty())
            .unwrap_or_else(|| "<browser>".to_owned())
    }

    impl CefariApp {
        fn build(
            browser_config: BrowserConfig,
            router_config: MessageRouterConfig,
            message_pump: SharedMessagePumpState,
        ) -> cef::App {
            Self::new(
                browser_config,
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
            downloads: SharedDownloadState,
        ) -> cef::Client {
            Self::new(
                browser_router.clone(),
                CefariDownloadHandler::new(downloads),
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
            downloads: SharedDownloadState,
        }

        impl DownloadHandler {
            fn can_download(
                &self,
                _browser: Option<&mut cef::Browser>,
                url: Option<&cef::CefString>,
                request_method: Option<&cef::CefString>,
            ) -> ::std::os::raw::c_int {
                let url = optional_cef_string(url);
                match DownloadPolicy::decide(&url) {
                    DownloadDecision::Allow => {
                        debug!(
                            url,
                            request_method = %optional_cef_string(request_method),
                            "allowed CEF download"
                        );
                        1
                    }
                    DownloadDecision::Deny(reason) => {
                        warn!(
                            url,
                            request_method = %optional_cef_string(request_method),
                            reason,
                            "denied CEF download"
                        );
                        0
                    }
                }
            }

            fn on_before_download(
                &self,
                browser: Option<&mut cef::Browser>,
                download_item: Option<&mut cef::DownloadItem>,
                suggested_name: Option<&cef::CefString>,
                callback: Option<&mut cef::BeforeDownloadCallback>,
            ) -> ::std::os::raw::c_int {
                let Some(download_item) = download_item else {
                    warn!("denied CEF download before start because download item is missing");
                    return 0;
                };
                let snapshot = download_snapshot(download_item, suggested_name);
                match DownloadPolicy::decide(&snapshot.url) {
                    DownloadDecision::Allow => {
                        emit_browser_event(browser.as_deref(), &self.downloads.start(&snapshot));
                        if let Some(callback) = callback {
                            callback.cont(None, 1);
                            return 1;
                        }
                        warn!("denied CEF download before start because callback is missing");
                        0
                    }
                    DownloadDecision::Deny(reason) => {
                        warn!(url = %snapshot.url, reason, "denied CEF download before start");
                        0
                    }
                }
            }

            fn on_download_updated(
                &self,
                browser: Option<&mut cef::Browser>,
                download_item: Option<&mut cef::DownloadItem>,
                callback: Option<&mut cef::DownloadItemCallback>,
            ) {
                let Some(download_item) = download_item else {
                    warn!("ignored CEF download update because download item is missing");
                    return;
                };
                let snapshot = download_snapshot(download_item, None);
                let callback = callback.cloned();
                if let Some(event) = self.downloads.update(&snapshot, callback) {
                    emit_browser_event(browser.as_deref(), &event);
                }
            }
        }
    }

    fn download_snapshot(
        download_item: &cef::DownloadItem,
        suggested_name: Option<&cef::CefString>,
    ) -> DownloadSnapshot {
        let id = format!("cef-{}", download_item.id());
        let suggested_name = optional_cef_string(suggested_name);
        let item_suggested_name = cef_userfree_string(&download_item.suggested_file_name());
        let full_path = cef_userfree_string(&download_item.full_path());
        let total_bytes = download_item.total_bytes();
        let percent_complete = download_item.percent_complete();
        DownloadSnapshot {
            id,
            url: cef_userfree_string(&download_item.url()),
            suggested_name: if suggested_name.is_empty() {
                item_suggested_name
            } else {
                suggested_name
            },
            destination_path: (!full_path.is_empty()).then_some(full_path),
            received_bytes: download_item.received_bytes(),
            total_bytes: (total_bytes > 0).then_some(total_bytes),
            percent_complete: (percent_complete >= 0).then_some(percent_complete),
            is_complete: download_item.is_complete() != 0,
            is_canceled: download_item.is_canceled() != 0,
            is_interrupted: download_item.is_interrupted() != 0,
            interrupt_reason: format!("{:?}", download_item.interrupt_reason()),
        }
    }

    fn emit_browser_event(browser: Option<&cef::Browser>, event: &CefariIpcEvent) {
        let Some(browser) = browser else {
            warn!(
                ?event,
                "failed to emit Cefari event because CEF browser is missing"
            );
            return;
        };
        let Some(frame) = browser.main_frame() else {
            warn!(
                ?event,
                "failed to emit Cefari event because CEF main frame is missing"
            );
            return;
        };
        let Ok(event_json) = serde_json::to_string(event) else {
            warn!(
                ?event,
                "failed to serialize Cefari event for browser delivery"
            );
            return;
        };
        let code =
            format!("if (window.__CEFARI_IPC_EVENT__) window.__CEFARI_IPC_EVENT__({event_json});");
        let code = cef::CefString::from(code.as_str());
        let script_url = cef::CefString::from("cefari://bridge/event.js");
        frame.execute_java_script(Some(&code), Some(&script_url), 1);
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
                    .map(|frame| cef_userfree_string(&frame.url()))
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

    pub fn initialize(
        paths: &cefari_core::RuntimePaths,
        browser_config: &cefari_core::BrowserConfig,
        devtools_endpoint: Option<crate::desktop_devtools::DevtoolsEndpoint>,
    ) -> Result<CefRuntime> {
        CefRuntime::initialize(paths, browser_config, devtools_endpoint)
            .context("failed to initialize CEF")
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
                Ok(cef::sys::HWND(handle.hwnd.get() as *mut cef::sys::HWND__))
            }
            #[cfg(all(unix, not(target_os = "macos")))]
            RawWindowHandle::Xlib(handle) => Ok(handle.window as cef::sys::cef_window_handle_t),
            other => anyhow::bail!("unsupported native window handle for CEF: {other:?}"),
        }
    }

    fn browser_bounds_for_window(window: &Window) -> cef::Rect {
        browser_bounds_for_size(window.inner_size(), window.scale_factor())
    }

    fn browser_bounds_for_size(size: PhysicalSize<u32>, scale_factor: f64) -> cef::Rect {
        cef::Rect {
            x: 0,
            y: 0,
            width: cef_bounds_dimension(size.width, scale_factor),
            height: cef_bounds_dimension(size.height, scale_factor),
        }
    }

    #[cfg(target_os = "macos")]
    #[allow(clippy::cast_possible_truncation)]
    fn cef_bounds_dimension(physical_size: u32, scale_factor: f64) -> i32 {
        if scale_factor.is_finite() && scale_factor > 0.0 {
            return (f64::from(physical_size) / scale_factor)
                .round()
                .clamp(0.0, f64::from(i32::MAX)) as i32;
        }

        i32::try_from(physical_size).unwrap_or(i32::MAX)
    }

    #[cfg(not(target_os = "macos"))]
    fn cef_bounds_dimension(physical_size: u32, _scale_factor: f64) -> i32 {
        i32::try_from(physical_size).unwrap_or(i32::MAX)
    }

    #[cfg(test)]
    mod tests {
        use tao::dpi::PhysicalSize;

        use super::{
            SharedBrowserState, append_chromium_feature_value, browser_bounds_for_size,
            development_chromium_switches_requested_from,
        };

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

        #[test]
        #[cfg(target_os = "macos")]
        fn browser_bounds_use_appkit_points_on_macos() {
            let bounds = browser_bounds_for_size(PhysicalSize::new(2400, 1600), 2.0);

            assert_eq!(bounds.x, 0);
            assert_eq!(bounds.y, 0);
            assert_eq!(bounds.width, 1200);
            assert_eq!(bounds.height, 800);
        }

        #[cfg(not(target_os = "macos"))]
        fn browser_bounds_match_window_inner_size() {
            let bounds = browser_bounds_for_size(PhysicalSize::new(1200, 800), 2.0);

            assert_eq!(bounds.x, 0);
            assert_eq!(bounds.y, 0);
            assert_eq!(bounds.width, 1200);
            assert_eq!(bounds.height, 800);
        }

        #[test]
        fn browser_bounds_clamp_dimensions_to_cef_rect_limits() {
            let bounds = browser_bounds_for_size(PhysicalSize::new(u32::MAX, u32::MAX), 1.0);

            assert_eq!(bounds.width, i32::MAX);
            assert_eq!(bounds.height, i32::MAX);
        }

        #[test]
        fn development_chromium_switches_are_dev_or_smoke_only() {
            assert!(development_chromium_switches_requested_from(true, false));
            assert!(development_chromium_switches_requested_from(false, true));
            assert!(!development_chromium_switches_requested_from(false, false));
        }

        #[test]
        fn chromium_feature_append_preserves_existing_features() {
            assert_eq!(append_chromium_feature_value("", "Vulkan"), "Vulkan");
            assert_eq!(
                append_chromium_feature_value("Foo, Vulkan,Bar", "Vulkan"),
                "Foo,Vulkan,Bar"
            );
            assert_eq!(
                append_chromium_feature_value("Foo,Bar", "Vulkan"),
                "Foo,Bar,Vulkan"
            );
        }
    }
}

pub use imp::{BridgeIpcSender, CefBridgeIpcRequest, CefRuntime, MessagePumpScheduler};

pub fn initialize(
    paths: &cefari_core::RuntimePaths,
    browser_config: &cefari_core::BrowserConfig,
    devtools_endpoint: Option<crate::desktop_devtools::DevtoolsEndpoint>,
) -> anyhow::Result<CefRuntime> {
    let runtime = imp::initialize(paths, browser_config, devtools_endpoint)?;
    tracing::info!("CEF runtime prepared");
    Ok(runtime)
}
