use anyhow::{Context, Result};
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
        MAIN_WINDOW_HEIGHT, MAIN_WINDOW_TITLE, MAIN_WINDOW_WIDTH, MIN_WINDOW_HEIGHT,
        MIN_WINDOW_WIDTH, main_window_builder,
    };

    #[test]
    fn main_window_spec_is_large_enough_for_desktop_shell() {
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
