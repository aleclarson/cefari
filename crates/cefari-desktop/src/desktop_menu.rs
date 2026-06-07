use std::path::Path;

use anyhow::{Context, Result};
use muda::{
    AboutMetadata, Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem, Submenu,
    accelerator::{Accelerator, CMD_OR_CTRL, Code},
};
use tracing::info;
#[cfg(not(target_os = "macos"))]
use tracing::{debug, warn};

use crate::external;

pub const CHECK_FOR_UPDATES_ID: &str = "cefari.menu.check_for_updates";
pub const OPEN_LOGS_ID: &str = "cefari.menu.open_logs";
pub const RELOAD_UI_ID: &str = "cefari.menu.reload_ui";
pub const SERVICE_STATUS_ID: &str = "cefari.menu.service_status";
pub const QUIT_ID: &str = "cefari.menu.quit";
#[cfg(test)]
const ROOT_MENU_LABELS: [&str; 6] = ["Cefari", "File", "Edit", "View", "Window", "Help"];

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MenuCommand {
    CheckForUpdates,
    OpenLogs,
    ReloadUi,
    ServiceStatus,
    Quit,
    Unhandled,
}

pub struct DesktopMenu {
    menu: Menu,
    #[cfg(target_os = "macos")]
    window_menu: Submenu,
    #[cfg(target_os = "macos")]
    help_menu: Submenu,
}

impl DesktopMenu {
    pub fn new() -> Result<Self> {
        let app_menu = app_menu()?;
        let file_menu = file_menu()?;
        let edit_menu = edit_menu()?;
        let view_menu = view_menu()?;
        let window_menu = window_menu()?;
        let help_menu = help_menu()?;
        let menu = Menu::with_items(&[
            &app_menu,
            &file_menu,
            &edit_menu,
            &view_menu,
            &window_menu,
            &help_menu,
        ])
        .context("failed to build Cefari desktop menu")?;

        Ok(Self {
            menu,
            #[cfg(target_os = "macos")]
            window_menu,
            #[cfg(target_os = "macos")]
            help_menu,
        })
    }

    pub fn install(&self) {
        install_platform_menu(self);
    }
}

pub fn command_for_event(event: &MenuEvent) -> MenuCommand {
    command_for_id(event.id())
}

fn command_for_id(id: &MenuId) -> MenuCommand {
    match id.as_ref() {
        CHECK_FOR_UPDATES_ID => MenuCommand::CheckForUpdates,
        OPEN_LOGS_ID => MenuCommand::OpenLogs,
        RELOAD_UI_ID => MenuCommand::ReloadUi,
        SERVICE_STATUS_ID => MenuCommand::ServiceStatus,
        QUIT_ID => MenuCommand::Quit,
        _ => MenuCommand::Unhandled,
    }
}

pub fn handle_menu_event(event: &MenuEvent, logs_dir: &Path) -> Result<MenuCommand> {
    let command = command_for_event(event);

    match command {
        MenuCommand::OpenLogs => {
            external::open_external_file(logs_dir)?;
        }
        MenuCommand::CheckForUpdates => {
            info!("update check requested from desktop menu");
        }
        MenuCommand::ReloadUi => {
            info!("UI reload requested from desktop menu");
        }
        MenuCommand::ServiceStatus => {
            info!("service status requested from desktop menu");
        }
        MenuCommand::Quit | MenuCommand::Unhandled => {}
    }

    Ok(command)
}

fn app_menu() -> Result<Submenu> {
    let about = PredefinedMenuItem::about(
        Some("About Cefari"),
        Some(AboutMetadata {
            name: Some("Cefari".to_string()),
            version: Some(env!("CARGO_PKG_VERSION").to_string()),
            ..Default::default()
        }),
    );
    let check_updates = MenuItem::with_id(
        CHECK_FOR_UPDATES_ID,
        "Check for Updates...",
        true,
        Some(accelerator("CmdOrCtrl+U")?),
    );
    let open_logs = MenuItem::with_id(
        OPEN_LOGS_ID,
        "Open Logs",
        true,
        Some(accelerator("CmdOrCtrl+L")?),
    );
    let quit = MenuItem::with_id(
        QUIT_ID,
        "Quit Cefari",
        true,
        Some(Accelerator::new(Some(CMD_OR_CTRL), Code::KeyQ)),
    );

    Submenu::with_items(
        "Cefari",
        true,
        &[
            &about,
            &PredefinedMenuItem::separator(),
            &check_updates,
            &open_logs,
            &PredefinedMenuItem::separator(),
            &quit,
        ],
    )
    .context("failed to build Cefari application menu")
}

fn file_menu() -> Result<Submenu> {
    Submenu::with_items(
        "File",
        true,
        &[
            &MenuItem::with_id(
                RELOAD_UI_ID,
                "Reload UI",
                true,
                Some(accelerator("CmdOrCtrl+R")?),
            ),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::close_window(None),
        ],
    )
    .context("failed to build Cefari file menu")
}

fn edit_menu() -> Result<Submenu> {
    Submenu::with_items(
        "Edit",
        true,
        &[
            &PredefinedMenuItem::undo(None),
            &PredefinedMenuItem::redo(None),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::cut(None),
            &PredefinedMenuItem::copy(None),
            &PredefinedMenuItem::paste(None),
            &PredefinedMenuItem::select_all(None),
        ],
    )
    .context("failed to build Cefari edit menu")
}

fn view_menu() -> Result<Submenu> {
    Submenu::with_items(
        "View",
        true,
        &[&MenuItem::with_id(
            SERVICE_STATUS_ID,
            "Service Status",
            true,
            None,
        )],
    )
    .context("failed to build Cefari view menu")
}

fn window_menu() -> Result<Submenu> {
    Submenu::with_items(
        "Window",
        true,
        &[
            &PredefinedMenuItem::minimize(None),
            &PredefinedMenuItem::maximize(None),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::bring_all_to_front(None),
        ],
    )
    .context("failed to build Cefari window menu")
}

fn help_menu() -> Result<Submenu> {
    Submenu::with_items("Help", true, &[&MenuItem::new("Cefari Help", false, None)])
        .context("failed to build Cefari help menu")
}

fn accelerator(shortcut: &str) -> Result<Accelerator> {
    shortcut
        .parse()
        .with_context(|| format!("failed to parse menu accelerator {shortcut}"))
}

#[cfg(target_os = "macos")]
fn install_platform_menu(menu: &DesktopMenu) {
    menu.menu.init_for_nsapp();
    menu.window_menu.set_as_windows_menu_for_nsapp();
    menu.help_menu.set_as_help_menu_for_nsapp();
    info!("installed native macOS menu");
}

#[cfg(not(target_os = "macos"))]
fn install_platform_menu(menu: &DesktopMenu) {
    let root_items = menu.menu.items().len();
    debug!(root_items, "built native menu model");
    warn!("native menu attachment is currently verified on macOS only");
}

#[cfg(test)]
mod tests {
    use muda::MenuId;

    use super::{
        CHECK_FOR_UPDATES_ID, MenuCommand, OPEN_LOGS_ID, QUIT_ID, RELOAD_UI_ID, ROOT_MENU_LABELS,
        SERVICE_STATUS_ID, command_for_id,
    };

    #[test]
    fn desktop_menu_spec_has_expected_root_items() {
        assert_eq!(
            ROOT_MENU_LABELS,
            ["Cefari", "File", "Edit", "View", "Window", "Help"]
        );
    }

    #[test]
    fn command_ids_map_to_known_menu_commands() {
        assert_eq!(
            command_for_id(&MenuId::new(CHECK_FOR_UPDATES_ID)),
            MenuCommand::CheckForUpdates
        );
        assert_eq!(
            command_for_id(&MenuId::new(OPEN_LOGS_ID)),
            MenuCommand::OpenLogs
        );
        assert_eq!(
            command_for_id(&MenuId::new(RELOAD_UI_ID)),
            MenuCommand::ReloadUi
        );
        assert_eq!(
            command_for_id(&MenuId::new(SERVICE_STATUS_ID)),
            MenuCommand::ServiceStatus
        );
        assert_eq!(command_for_id(&MenuId::new(QUIT_ID)), MenuCommand::Quit);
        assert_eq!(
            command_for_id(&MenuId::new("cefari.menu.unknown")),
            MenuCommand::Unhandled
        );
    }
}
