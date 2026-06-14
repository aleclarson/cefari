use std::{
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use cefari_core::{
    AppConfig, CefariIpcCommand, PackageFormat, RuntimePaths, packaged_resources_dir,
};
use png::{ColorType, Transformations};
use tracing::{debug, info};
use tray_icon::{
    Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuItem, PredefinedMenuItem},
};

use crate::desktop_menu::{CHECK_FOR_UPDATES_ID, OPEN_LOGS_ID, QUIT_ID};

const CEFARI_TRAY_ICON_ENV: &str = "CEFARI_TRAY_ICON";
const PACKAGED_TRAY_ICON: &str = "tray-icon.png";

pub struct DesktopTray {
    _tray_icon: TrayIcon,
}

impl DesktopTray {
    pub fn new(app_config: &AppConfig, paths: &RuntimePaths) -> Result<Self> {
        let menu = tray_menu(app_config)?;
        let icon = tray_icon(paths).context("failed to create Cefari tray icon")?;
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

pub(crate) fn tray_enabled(paths: &RuntimePaths) -> bool {
    tray_icon_candidates(paths)
        .into_iter()
        .any(|candidate| candidate.is_file())
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

fn tray_icon(paths: &RuntimePaths) -> Result<Icon> {
    let icon_path = tray_icon_path(paths)?;
    let (rgba, width, height) = decode_tray_icon_png(&icon_path)
        .with_context(|| format!("failed to decode tray icon at {}", icon_path.display()))?;
    Icon::from_rgba(rgba, width, height).map_err(|error| anyhow::anyhow!("{error}"))
}

fn tray_icon_path(paths: &RuntimePaths) -> Result<PathBuf> {
    for candidate in tray_icon_candidates(paths) {
        if candidate.is_file() {
            return Ok(candidate);
        }
    }

    anyhow::bail!(
        "tray icon is required; configure app.tray_icon and package it as {PACKAGED_TRAY_ICON}"
    )
}

fn tray_icon_candidates(paths: &RuntimePaths) -> Vec<PathBuf> {
    let mut candidates = std::env::var_os(CEFARI_TRAY_ICON_ENV)
        .map(PathBuf::from)
        .into_iter()
        .collect::<Vec<_>>();

    candidates.extend(platform_package_formats().iter().filter_map(|format| {
        packaged_resources_dir(*format)
            .ok()
            .map(|resources_dir| resources_dir.join(PACKAGED_TRAY_ICON))
    }));
    candidates.push(paths.resource_dir.join(PACKAGED_TRAY_ICON));
    candidates
}

fn decode_tray_icon_png(path: &Path) -> Result<(Vec<u8>, u32, u32)> {
    let file = File::open(path)
        .with_context(|| format!("failed to open tray icon at {}", path.display()))?;
    let mut decoder = png::Decoder::new(BufReader::new(file));
    decoder.set_transformations(Transformations::normalize_to_color8() | Transformations::ALPHA);
    let mut reader = decoder
        .read_info()
        .with_context(|| format!("failed to read tray icon metadata at {}", path.display()))?;
    let buffer_size = reader
        .output_buffer_size()
        .context("tray icon PNG is too large to decode")?;
    let mut buffer = vec![0; buffer_size];
    let output = reader
        .next_frame(&mut buffer)
        .with_context(|| format!("failed to read tray icon pixels at {}", path.display()))?;
    let pixels = &buffer[..output.buffer_size()];
    let rgba = pixels_to_rgba(pixels, output.color_type)?;
    Ok((rgba, output.width, output.height))
}

fn pixels_to_rgba(pixels: &[u8], color_type: ColorType) -> Result<Vec<u8>> {
    match color_type {
        ColorType::Rgba => Ok(pixels.to_vec()),
        ColorType::Rgb => {
            let mut rgba = Vec::with_capacity(pixels.len() / 3 * 4);
            for pixel in pixels.chunks_exact(3) {
                rgba.extend_from_slice(&[pixel[0], pixel[1], pixel[2], 255]);
            }
            Ok(rgba)
        }
        ColorType::Grayscale => {
            let mut rgba = Vec::with_capacity(pixels.len() * 4);
            for &gray in pixels {
                rgba.extend_from_slice(&[gray, gray, gray, 255]);
            }
            Ok(rgba)
        }
        ColorType::GrayscaleAlpha => {
            let mut rgba = Vec::with_capacity(pixels.len() / 2 * 4);
            for pixel in pixels.chunks_exact(2) {
                rgba.extend_from_slice(&[pixel[0], pixel[0], pixel[0], pixel[1]]);
            }
            Ok(rgba)
        }
        ColorType::Indexed => anyhow::bail!("indexed tray icon did not expand to RGBA"),
    }
}

#[cfg(target_os = "macos")]
fn platform_package_formats() -> &'static [PackageFormat] {
    &[PackageFormat::App, PackageFormat::Dmg]
}

#[cfg(target_os = "windows")]
fn platform_package_formats() -> &'static [PackageFormat] {
    &[PackageFormat::Nsis, PackageFormat::Wix]
}

#[cfg(all(unix, not(target_os = "macos")))]
fn platform_package_formats() -> &'static [PackageFormat] {
    &[
        PackageFormat::Deb,
        PackageFormat::AppImage,
        PackageFormat::Pacman,
    ]
}

#[cfg(not(any(target_os = "macos", target_os = "windows", unix)))]
fn platform_package_formats() -> &'static [PackageFormat] {
    &[]
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use cefari_core::{AppConfig, RuntimePaths};

    use super::{
        PACKAGED_TRAY_ICON, decode_tray_icon_png, ipc_command_for_event, tray_enabled,
        tray_quit_label,
    };

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
    fn decodes_png_tray_icon() {
        let icon_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../cefari-cli/assets/default-tray-icon.png");
        let (rgba, width, height) =
            decode_tray_icon_png(&icon_path).expect("tray icon should decode");

        assert_eq!((width, height), (18, 18));
        assert_eq!(rgba.len(), 18 * 18 * 4);
        assert!(rgba.chunks_exact(4).any(|pixel| pixel[3] == 255));
        assert!(rgba.chunks_exact(4).any(|pixel| pixel[3] == 0));
    }

    #[test]
    fn tray_is_enabled_when_resource_icon_exists() {
        let root = temp_dir("tray-enabled");
        let paths = runtime_paths(&root);

        assert!(!tray_enabled(&paths));

        fs::create_dir_all(&paths.resource_dir).expect("resource dir should be created");
        fs::write(paths.resource_dir.join(PACKAGED_TRAY_ICON), "icon")
            .expect("tray icon should be written");

        assert!(tray_enabled(&paths));

        fs::remove_dir_all(root).expect("temp dir should be removable");
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

    fn runtime_paths(root: &Path) -> RuntimePaths {
        RuntimePaths {
            config_dir: root.join("config"),
            config_file: root.join("config/cefari.json"),
            data_dir: root.join("data"),
            cache_dir: root.join("cache"),
            log_dir: root.join("logs"),
            resource_dir: root.join("resources"),
            update_dir: root.join("updates"),
        }
    }

    fn temp_dir(label: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("cefari-desktop-{label}-{suffix}"))
    }
}
