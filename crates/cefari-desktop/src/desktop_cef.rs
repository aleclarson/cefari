#[cfg(feature = "cef")]
mod imp {
    use std::ptr;

    use anyhow::{Context, Result};
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
