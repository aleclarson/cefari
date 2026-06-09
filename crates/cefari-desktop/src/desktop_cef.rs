#[cfg(feature = "cef")]
mod imp {
    use std::ptr;

    use anyhow::{Context, Result};
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use tao::window::Window;
    use tracing::info;

    pub struct CefRuntime {
        initialized: bool,
    }

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

            info!("CEF initialized");
            Ok(Self { initialized: true })
        }

        pub fn create_browser(&self, window: &Window, url: &str) -> Result<()> {
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
                None,
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

        pub fn pump_message_loop(&self) {
            if self.initialized {
                cef::do_message_loop_work();
            }
        }
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

    impl CefRuntime {
        pub fn initialize() -> Self {
            info!("CEF feature disabled; skipping CEF initialization");
            Self { enabled: false }
        }

        pub fn create_browser(
            &self,
            _window: &tao::window::Window,
            _url: &str,
        ) -> anyhow::Result<()> {
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
