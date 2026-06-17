use anyhow::{Context, Result};
use cefari_core::WindowState;
use tao::{
    dpi::LogicalSize,
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};
use tracing::error;

use crate::{desktop_ui, event_loop::UserEvent};

const MAIN_WINDOW_TITLE: &str = "Cefari";
const MAIN_WINDOW_WIDTH: f64 = 1200.0;
const MAIN_WINDOW_HEIGHT: f64 = 800.0;
const MIN_WINDOW_WIDTH: f64 = 800.0;
const MIN_WINDOW_HEIGHT: f64 = 560.0;

pub(crate) const MAIN_WINDOW_ID: &str = "main";

pub(crate) struct WindowManager {
    main: Option<Window>,
    main_title: String,
}

impl WindowManager {
    pub(crate) fn with_main(window: Window) -> Self {
        Self {
            main: Some(window),
            main_title: default_window_title(),
        }
    }

    pub(crate) fn main_window(&self) -> Result<&Window> {
        self.main
            .as_ref()
            .context("main window is no longer available")
    }

    pub(crate) fn show_main(&mut self) -> Result<WindowState> {
        let window = self.main_window()?;
        window.set_visible(true);
        Ok(self.main_state())
    }

    pub(crate) fn focus_main(&mut self) -> Result<WindowState> {
        let window = self.main_window()?;
        window.set_visible(true);
        window.set_focus();
        Ok(self.main_state())
    }

    pub(crate) fn close_main(&mut self) -> WindowState {
        self.main = None;
        self.main_state()
    }

    pub(crate) fn set_main_title(&mut self, title: &str) -> Result<WindowState> {
        let window = self.main_window()?;
        window.set_title(title);
        title.clone_into(&mut self.main_title);
        Ok(self.main_state())
    }

    pub(crate) fn main_state(&self) -> WindowState {
        WindowState {
            visible: self.main.as_ref().is_some_and(Window::is_visible),
            focused: self.main.as_ref().is_some_and(Window::is_focused),
            title: self.main_title.clone(),
        }
    }
}

pub(crate) fn default_window_title() -> String {
    MAIN_WINDOW_TITLE.to_owned()
}

pub(crate) fn apply_ui_diagnostic_state(window: &Window, shell_ui: &desktop_ui::ShellUi) {
    if shell_ui.is_diagnostic() {
        window.set_title("Cefari - Missing UI Resources");
        error!(ui_entry = %shell_ui.entry_path.display(), "using diagnostic UI fallback");
    }
}

pub(crate) fn create_main_window(
    event_loop: &EventLoop<UserEvent>,
    background_smoke: bool,
) -> Result<Window> {
    main_window_builder(background_smoke)
        .build(event_loop)
        .context("failed to create Cefari main window")
}

fn main_window_builder(background_smoke: bool) -> WindowBuilder {
    let builder = WindowBuilder::new()
        .with_title(MAIN_WINDOW_TITLE)
        .with_inner_size(LogicalSize::new(MAIN_WINDOW_WIDTH, MAIN_WINDOW_HEIGHT))
        .with_min_inner_size(LogicalSize::new(MIN_WINDOW_WIDTH, MIN_WINDOW_HEIGHT));

    if background_smoke {
        return builder.with_visible(false).with_focused(false);
    }

    builder
}

#[cfg(test)]
mod tests {
    use super::{
        MAIN_WINDOW_HEIGHT, MAIN_WINDOW_ID, MAIN_WINDOW_TITLE, MAIN_WINDOW_WIDTH,
        MIN_WINDOW_HEIGHT, MIN_WINDOW_WIDTH, main_window_builder,
    };

    #[test]
    fn main_window_spec_is_large_enough_for_desktop_shell() {
        assert_eq!(MAIN_WINDOW_ID, "main");
        assert_eq!(MAIN_WINDOW_TITLE, "Cefari");
        assert!(std::hint::black_box(MAIN_WINDOW_WIDTH) >= std::hint::black_box(MIN_WINDOW_WIDTH));
        assert!(
            std::hint::black_box(MAIN_WINDOW_HEIGHT) >= std::hint::black_box(MIN_WINDOW_HEIGHT)
        );
    }

    #[test]
    fn smoke_background_window_starts_hidden_and_unfocused() {
        let normal = main_window_builder(false);
        let background = main_window_builder(true);

        assert!(normal.window.visible);
        assert!(normal.window.focused);
        assert!(!background.window.visible);
        assert!(!background.window.focused);
    }
}
