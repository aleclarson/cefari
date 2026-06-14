use anyhow::{Context, Result};
use cefari_core::{AppConfig, CefariIpcCommand};
use tracing::{debug, info};
use tray_icon::{
    Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuItem, PredefinedMenuItem},
};

use crate::desktop_menu::{CHECK_FOR_UPDATES_ID, OPEN_LOGS_ID, QUIT_ID};

const TRAY_ICON_SIZE: u32 = 18;
const TRAY_ICON_PIXEL_COUNT: usize = (TRAY_ICON_SIZE * TRAY_ICON_SIZE) as usize;

pub struct DesktopTray {
    _tray_icon: TrayIcon,
}

impl DesktopTray {
    pub fn new(app_config: &AppConfig) -> Result<Self> {
        let menu = tray_menu(app_config)?;
        let icon = tray_icon().context("failed to create Cefari tray icon")?;
        let tray_icon = TrayIconBuilder::new()
            .with_tooltip(&app_config.display_name)
            .with_icon(icon)
            .with_icon_as_template(true)
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .with_menu_on_right_click(true)
            .build()
            .context("failed to create Cefari tray icon")?;

        info!("cefari tray icon created");
        Ok(Self {
            _tray_icon: tray_icon,
        })
    }
}

pub fn ipc_command_for_event(event: &TrayIconEvent) -> Option<CefariIpcCommand> {
    if is_restore_window_event(event) {
        Some(CefariIpcCommand::TrayRestoreWindow)
    } else {
        None
    }
}

pub fn log_tray_event(event: &TrayIconEvent) {
    debug!(?event, "received tray icon event");
}

fn is_restore_window_event(event: &TrayIconEvent) -> bool {
    matches!(
        event,
        TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        } | TrayIconEvent::DoubleClick {
            button: MouseButton::Left,
            ..
        }
    )
}

fn tray_menu(app_config: &AppConfig) -> Result<Menu> {
    let quit_label = tray_quit_label(app_config);
    let menu = Menu::with_items(&[
        &MenuItem::with_id(OPEN_LOGS_ID, "Open Logs", true, None),
        &MenuItem::with_id(CHECK_FOR_UPDATES_ID, "Check for Updates...", true, None),
        &PredefinedMenuItem::separator(),
        &MenuItem::with_id(QUIT_ID, quit_label, true, None),
    ])
    .context("failed to build Cefari tray menu")?;

    Ok(menu)
}

fn tray_quit_label(app_config: &AppConfig) -> String {
    format!("Quit {}", app_config.display_name)
}

fn tray_icon() -> Result<Icon> {
    Icon::from_rgba(tray_icon_rgba(), TRAY_ICON_SIZE, TRAY_ICON_SIZE)
        .map_err(|error| anyhow::anyhow!("{error}"))
}

fn tray_icon_rgba() -> Vec<u8> {
    let mut rgba = Vec::with_capacity(TRAY_ICON_PIXEL_COUNT * 4);
    let center = f64::from(TRAY_ICON_SIZE - 1) / 2.0;
    let outer_radius = center;
    let inner_radius = center * 0.45;

    for y in 0..TRAY_ICON_SIZE {
        for x in 0..TRAY_ICON_SIZE {
            let dx = f64::from(x) - center;
            let dy = f64::from(y) - center;
            let distance = dx.hypot(dy);
            let alpha = if distance <= inner_radius {
                0
            } else if distance <= outer_radius {
                255
            } else {
                0
            };

            rgba.extend_from_slice(&[0, 0, 0, alpha]);
        }
    }

    rgba
}

#[cfg(test)]
mod tests {
    use cefari_core::AppConfig;

    use super::{
        TRAY_ICON_PIXEL_COUNT, TRAY_ICON_SIZE, ipc_command_for_event, tray_icon_rgba,
        tray_quit_label,
    };

    #[test]
    fn tray_icon_rgba_has_expected_dimensions() {
        let rgba = tray_icon_rgba();

        assert_eq!(rgba.len(), TRAY_ICON_PIXEL_COUNT * 4);
        assert_eq!(TRAY_ICON_SIZE, 18);
        assert!(rgba.chunks_exact(4).any(|pixel| pixel[3] == 255));
        assert!(rgba.chunks_exact(4).any(|pixel| pixel[3] == 0));
    }

    #[test]
    fn tray_menu_labels_use_app_config() {
        let config = AppConfig {
            identifier: "dev.cefari.custom".to_owned(),
            display_name: "Custom App".to_owned(),
            version: "2.3.4".to_owned(),
        };

        assert_eq!(tray_quit_label(&config), "Quit Custom App");
    }

    #[test]
    fn non_primary_tray_events_do_not_emit_ipc_commands() {
        let event = tray_icon::TrayIconEvent::Click {
            id: tray_icon::TrayIconId("test".to_owned()),
            position: tray_icon::dpi::PhysicalPosition::new(0.0, 0.0),
            rect: tray_icon::Rect::default(),
            button: tray_icon::MouseButton::Right,
            button_state: tray_icon::MouseButtonState::Up,
        };

        assert_eq!(ipc_command_for_event(&event), None);
    }
}
