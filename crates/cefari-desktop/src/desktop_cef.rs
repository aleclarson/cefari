#[cfg(feature = "cef")]
mod imp {
    use std::{cell::RefCell, ptr, rc::Rc};

    use anyhow::{Context, Result};
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use tao::window::Window;
    use tracing::{debug, error, info, warn};

    use cef::rc::Rc as _;
    use cef::{
        Client, ImplBrowser as _, ImplBrowserHost as _, ImplClient, ImplFrame as _,
        ImplLifeSpanHandler, ImplLoadHandler, ImplProcessMessage as _, ImplRenderHandler,
        ImplRequest as _, ImplRequestHandler, LifeSpanHandler, LoadHandler, RenderHandler,
        RequestHandler, WrapClient, WrapLifeSpanHandler, WrapLoadHandler, WrapRenderHandler,
        WrapRequestHandler, wrap_client, wrap_life_span_handler, wrap_load_handler,
        wrap_render_handler, wrap_request_handler,
    };

    use crate::desktop_bridge::{BridgeOriginPolicy, origin_from_url};

    pub struct CefRuntime {
        initialized: bool,
        #[allow(dead_code)]
        state: SharedBrowserState,
        client: cef::Client,
    }

    #[allow(dead_code)]
    impl CefRuntime {
        pub fn initialize() -> Result<Self> {
            let args = cef::args::Args::new();
            let subprocess_exit =
                cef::execute_process(Some(args.as_main_args()), None, ptr::null_mut());

            if subprocess_exit >= 0 {
                info!(status = subprocess_exit, "CEF subprocess completed");
                std::process::exit(subprocess_exit);
            }

            let settings = cef::Settings {
                no_sandbox: 1,
                external_message_pump: 1,
                ..Default::default()
            };

            let initialized = cef::initialize(
                Some(args.as_main_args()),
                Some(&settings),
                None,
                ptr::null_mut(),
            );

            if initialized != 1 {
                anyhow::bail!("CEF initialization returned {initialized}");
            }

            let state = SharedBrowserState::default();

            info!("CEF initialized");
            Ok(Self {
                initialized: true,
                state: state.clone(),
                client: CefariCefClient::build(state, BridgeOriginPolicy::from_environment()),
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

        fn browser_host(&self) -> Result<cef::BrowserHost> {
            let browser = self.state.active_browser()?;
            browser
                .host()
                .with_context(|| format!("CEF browser {} has no host", browser.identifier()))
        }
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

    wrap_client! {
        struct CefariCefClient {
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

            fn on_process_message_received(
                &self,
                _browser: Option<&mut cef::Browser>,
                frame: Option<&mut cef::Frame>,
                source_process: cef::ProcessId,
                message: Option<&mut cef::ProcessMessage>,
            ) -> ::std::os::raw::c_int {
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
        fn build(state: SharedBrowserState, origin_policy: BridgeOriginPolicy) -> cef::Client {
            Self::new(
                CefariLifeSpanHandler::new(state),
                CefariLoadHandler::new(),
                CefariRenderHandler::new(),
                CefariRequestHandler::new(origin_policy),
            )
        }
    }

    wrap_life_span_handler! {
        struct CefariLifeSpanHandler {
            state: SharedBrowserState,
        }

        impl LifeSpanHandler {
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
        }

        impl RequestHandler {
            fn on_before_browse(
                &self,
                _browser: Option<&mut cef::Browser>,
                frame: Option<&mut cef::Frame>,
                request: Option<&mut cef::Request>,
                user_gesture: ::std::os::raw::c_int,
                is_redirect: ::std::os::raw::c_int,
            ) -> ::std::os::raw::c_int {
                debug!(
                    frame_url = %frame_url(frame),
                    request_url = %request_url(request),
                    user_gesture = user_gesture != 0,
                    is_redirect = is_redirect != 0,
                    "CEF navigation requested"
                );
                0
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
                _browser: Option<&mut cef::Browser>,
                status: cef::TerminationStatus,
                error_code: ::std::os::raw::c_int,
                error_string: Option<&cef::CefString>,
            ) {
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

    pub fn initialize() -> Result<CefRuntime> {
        CefRuntime::initialize().context("failed to initialize CEF")
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
            let error = match state.active_browser() {
                Ok(_) => panic!("empty state should not return a browser"),
                Err(error) => error,
            };

            assert!(
                error
                    .to_string()
                    .contains("CEF main browser is not available")
            );
        }
    }
}

#[cfg(feature = "cef")]
pub use imp::CefRuntime;

#[cfg(feature = "cef")]
pub fn initialize() -> anyhow::Result<CefRuntime> {
    let runtime = imp::initialize()?;
    tracing::info!("CEF runtime prepared");
    Ok(runtime)
}

#[cfg(not(feature = "cef"))]
mod imp {
    use tracing::info;

    pub struct CefRuntime {
        enabled: bool,
    }

    #[allow(dead_code)]
    impl CefRuntime {
        pub fn initialize() -> Self {
            info!("CEF feature disabled; skipping CEF initialization");
            Self { enabled: false }
        }

        pub fn create_browser(
            &mut self,
            _window: &tao::window::Window,
            _url: &str,
        ) -> anyhow::Result<()> {
            anyhow::bail!("CEF feature disabled; rebuild cefari-desktop with --features cef")
        }

        pub fn has_browser(&self) -> bool {
            false
        }

        pub fn browser_identifier(&self) -> anyhow::Result<i32> {
            anyhow::bail!("CEF feature disabled; rebuild cefari-desktop with --features cef")
        }

        pub fn reload_browser(&self) -> anyhow::Result<()> {
            anyhow::bail!("CEF feature disabled; rebuild cefari-desktop with --features cef")
        }

        pub fn focus_browser(&self, _focused: bool) -> anyhow::Result<()> {
            anyhow::bail!("CEF feature disabled; rebuild cefari-desktop with --features cef")
        }

        pub fn close_browser(&self, _force_close: bool) -> anyhow::Result<()> {
            anyhow::bail!("CEF feature disabled; rebuild cefari-desktop with --features cef")
        }

        pub fn notify_browser_resized(&self) -> anyhow::Result<()> {
            anyhow::bail!("CEF feature disabled; rebuild cefari-desktop with --features cef")
        }

        pub fn notify_browser_screen_info_changed(&self) -> anyhow::Result<()> {
            anyhow::bail!("CEF feature disabled; rebuild cefari-desktop with --features cef")
        }

        pub fn notify_browser_move_or_resize_started(&self) -> anyhow::Result<()> {
            anyhow::bail!("CEF feature disabled; rebuild cefari-desktop with --features cef")
        }

        pub fn pump_message_loop(&self) {
            if self.enabled {
                unreachable!("CEF cannot be enabled without the cef feature");
            }
        }
    }
}

#[cfg(not(feature = "cef"))]
pub use imp::CefRuntime;

#[cfg(not(feature = "cef"))]
pub fn initialize() -> CefRuntime {
    let runtime = CefRuntime::initialize();
    tracing::info!("CEF runtime prepared");
    runtime
}
