use std::process::ExitCode;

mod desktop_app;
mod desktop_bridge;
mod desktop_cef;
mod desktop_files;
mod desktop_ipc;
mod desktop_menu;
mod desktop_notifications;
mod desktop_single_instance;
mod desktop_tray;
mod desktop_ui;
mod event_loop;
mod external;
mod logging;
mod runtime;
mod shell_context;
mod window;

fn main() -> ExitCode {
    match desktop_app::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            desktop_app::report_startup_error(&error);
            ExitCode::FAILURE
        }
    }
}
