#[cfg(target_os = "macos")]
mod macos_helpers;
mod paths;

const CEFARI_DEV_MODE_ENV: &str = "CEFARI_DEV_MODE";
const CEFARI_DEVTOOLS_PORT_ENV: &str = "CEFARI_DEVTOOLS_PORT";
const CEFARI_SMOKE_BACKGROUND_ENV: &str = "CEFARI_SMOKE_BACKGROUND";

mod imp {
    #![allow(clippy::transmute_ptr_to_ptr)]

    mod runtime;
    mod state;

    use std::{
        fs,
        path::PathBuf,
        sync::{Arc, Mutex},
    };

    use anyhow::{Context, Result};
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use tao::dpi::PhysicalSize;
    use tao::window::Window;
    use tracing::{debug, error, info, warn};

    use cef::rc::Rc as _;
    use cef::wrapper::message_router::{
        BrowserSideCallback, BrowserSideHandler, BrowserSideRouter,
        MessageRouterBrowserSideHandlerCallbacks, MessageRouterConfig, MessageRouterRendererSide,
        MessageRouterRendererSideHandlerCallbacks, RendererSideRouter,
    };
    use cef::{
        App, BrowserProcessHandler, Client, DownloadHandler, ImplApp, ImplBrowser as _,
        ImplBrowserProcessHandler, ImplClient, ImplCommandLine as _, ImplDownloadHandler,
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
    use crate::desktop_ui::CEFARI_APP_SCHEME;
    use crate::external;

    use super::paths::CefRuntimePathConfig;
    pub use runtime::{BridgeIpcSender, CefBridgeIpcRequest, CefRuntime, MessagePumpScheduler};
    use state::{
        SharedAppSchemeState, SharedBridgeIpcState, SharedBrowserState, SharedMessagePumpState,
    };

    fn cef_settings(runtime_paths: &CefRuntimePathConfig) -> cef::Settings {
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
        if let Some(port) = devtools_port_from_environment() {
            settings.remote_debugging_port = i32::from(port);
            info!(
                port,
                "enabled CEF Chrome DevTools Protocol remote debugging"
            );
        }

        settings
    }

    fn devtools_port_from_environment() -> Option<u16> {
        if std::env::var(super::CEFARI_DEV_MODE_ENV).as_deref() != Ok("1") {
            return None;
        }
        let port = std::env::var(super::CEFARI_DEVTOOLS_PORT_ENV).ok()?;
        parse_devtools_port(&port)
    }

    fn parse_devtools_port(port: &str) -> Option<u16> {
        port.parse::<u16>().ok().filter(|port| *port != 0)
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
            fn on_before_command_line_processing(
                &self,
                process_type: Option<&cef::CefString>,
                command_line: Option<&mut cef::CommandLine>,
            ) {
                configure_development_chromium_command_line(process_type, command_line);
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

    fn configure_development_chromium_command_line(
        process_type: Option<&cef::CefString>,
        command_line: Option<&mut cef::CommandLine>,
    ) {
        if !development_chromium_switches_requested() {
            return;
        }
        let Some(command_line) = command_line else {
            return;
        };

        append_chromium_switch(command_line, "use-mock-keychain");
        append_chromium_switch_with_value(command_line, "password-store", "basic");
        append_chromium_switch(command_line, "disable-save-password-bubble");
        append_chromium_switch(command_line, "disable-notifications");
        append_chromium_switch(command_line, "deny-permission-prompts");

        debug!(
            process_type = %display_cef_process_type(process_type),
            "configured development Chromium command line"
        );
    }

    fn development_chromium_switches_requested() -> bool {
        development_chromium_switches_requested_from(
            std::env::var(super::CEFARI_DEV_MODE_ENV).as_deref() == Ok("1"),
            std::env::var(super::CEFARI_SMOKE_BACKGROUND_ENV).as_deref() == Ok("1"),
        )
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

    fn display_cef_process_type(process_type: Option<&cef::CefString>) -> String {
        process_type
            .and_then(cef::CefString::as_slice)
            .map(String::from_utf16_lossy)
            .filter(|process_type| !process_type.is_empty())
            .unwrap_or_else(|| "<browser>".to_owned())
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
                .map(|frame| cef_userfree_string(&frame.url()))
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

    fn cef_userfree_string(value: &cef::CefStringUserfree) -> String {
        let value: Option<&cef::sys::_cef_string_utf16_t> = value.into();
        let Some(value) = value else {
            return String::new();
        };
        if value.str_.is_null() || value.length == 0 {
            return String::new();
        }

        cef::CefString::from(*value).to_string()
    }

    fn frame_url(frame: Option<&mut cef::Frame>) -> String {
        frame
            .map(|frame| cef_userfree_string(&frame.url()))
            .unwrap_or_default()
    }

    fn request_url(request: Option<&mut cef::Request>) -> String {
        request
            .map(|request| cef_userfree_string(&request.url()))
            .unwrap_or_default()
    }

    fn message_name(message: Option<&mut cef::ProcessMessage>) -> String {
        message
            .map(|message| cef_userfree_string(&message.name()))
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

        let frame_url = cef_userfree_string(&frame.url());
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
            SharedBrowserState, browser_bounds_for_size,
            development_chromium_switches_requested_from, parse_devtools_port,
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
        fn parses_nonzero_devtools_ports() {
            assert_eq!(parse_devtools_port("9222"), Some(9222));
            assert_eq!(parse_devtools_port("0"), None);
            assert_eq!(parse_devtools_port("not-a-port"), None);
        }

        #[test]
        fn development_chromium_switches_are_dev_or_smoke_only() {
            assert!(development_chromium_switches_requested_from(true, false));
            assert!(development_chromium_switches_requested_from(false, true));
            assert!(!development_chromium_switches_requested_from(false, false));
        }
    }
}

pub use imp::{BridgeIpcSender, CefBridgeIpcRequest, CefRuntime, MessagePumpScheduler};

pub fn initialize(paths: &cefari_core::RuntimePaths) -> anyhow::Result<CefRuntime> {
    let runtime = imp::initialize(paths)?;
    tracing::info!("CEF runtime prepared");
    Ok(runtime)
}
