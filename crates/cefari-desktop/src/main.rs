use std::process::ExitCode;

mod desktop_app;
mod desktop_bridge;
mod desktop_cef;
mod desktop_daemon;
mod desktop_dialogs;
mod desktop_downloads;
mod desktop_files;
mod desktop_ipc;
mod desktop_menu;
mod desktop_notifications;
mod desktop_single_instance;
mod desktop_tray;
mod desktop_ui;
mod desktop_workers;
mod event_loop;
mod external;
mod logging;
mod runtime;
mod shell_context;
mod window;
mod window_state;

fn main() -> ExitCode {
    match desktop_app::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            desktop_app::report_startup_error(&error);
            ExitCode::FAILURE
        }
    }
}
