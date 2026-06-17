use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use cefari_core::RuntimePaths;
use serde::{Deserialize, Serialize};
use tao::{
    dpi::{PhysicalPosition, PhysicalSize},
    window::Window,
};
use tracing::{debug, warn};

pub(crate) const MAIN_WINDOW_PERSIST_KEY: &str = "main";

const WINDOW_STATE_FILE: &str = "window-state.json";
const WRITE_DEBOUNCE: Duration = Duration::from_millis(250);

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WindowGeometry {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) x: Option<i32>,
    pub(crate) y: Option<i32>,
    pub(crate) maximized: bool,
    pub(crate) fullscreen: bool,
}

#[derive(Debug, Default, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
struct PersistedWindowState {
    windows: BTreeMap<String, WindowGeometry>,
}

#[derive(Debug)]
pub(crate) struct WindowStateStore {
    path: PathBuf,
    state: PersistedWindowState,
    dirty: bool,
    flush_deadline: Option<Instant>,
}

impl WindowStateStore {
    pub(crate) fn load(paths: &RuntimePaths) -> Self {
        let path = state_file_path(paths);
        match read_state_file(&path) {
            Ok(state) => Self::from_state(path, valid_state(state)),
            Err(error) => {
                warn!(%error, path = %path.display(), "ignoring persisted window state");
                Self::from_state(path, PersistedWindowState::default())
            }
        }
    }

    pub(crate) fn geometry(&self, persist_key: &str) -> Option<&WindowGeometry> {
        self.state.windows.get(persist_key)
    }

    pub(crate) fn stage_window(&mut self, persist_key: &str, window: &Window) {
        self.state
            .windows
            .insert(persist_key.to_owned(), capture_window_geometry(window));
        self.dirty = true;
        self.flush_deadline = Some(Instant::now() + WRITE_DEBOUNCE);
    }

    pub(crate) fn flush_if_due(&mut self, now: Instant) {
        if self.flush_deadline.is_some_and(|deadline| deadline <= now) {
            if let Err(error) = self.flush() {
                warn!(%error, path = %self.path.display(), "failed to persist window state");
            }
        }
    }

    pub(crate) fn flush_deadline(&self) -> Option<Instant> {
        self.flush_deadline
    }

    pub(crate) fn flush(&mut self) -> Result<()> {
        if !self.dirty {
            self.flush_deadline = None;
            return Ok(());
        }

        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "failed to create window state directory at {}",
                    parent.display()
                )
            })?;
        }
        let json = serde_json::to_string_pretty(&self.state)
            .context("failed to serialize window state")?;
        fs::write(&self.path, json)
            .with_context(|| format!("failed to write window state to {}", self.path.display()))?;
        self.dirty = false;
        self.flush_deadline = None;
        debug!(path = %self.path.display(), "persisted window state");
        Ok(())
    }

    fn from_state(path: PathBuf, state: PersistedWindowState) -> Self {
        Self {
            path,
            state,
            dirty: false,
            flush_deadline: None,
        }
    }
}

pub(crate) fn persist_key_from_request(persist_key: Option<&str>) -> Option<String> {
    persist_key
        .map(str::trim)
        .filter(|key| !key.is_empty())
        .map(ToOwned::to_owned)
}

pub(crate) fn capture_window_geometry(window: &Window) -> WindowGeometry {
    let size = window.inner_size();
    let position = window.outer_position().ok();
    WindowGeometry {
        width: size.width,
        height: size.height,
        x: position.map(|position| position.x),
        y: position.map(|position| position.y),
        maximized: window.is_maximized(),
        fullscreen: window.fullscreen().is_some(),
    }
}

pub(crate) fn apply_geometry_to_builder(
    mut builder: tao::window::WindowBuilder,
    geometry: Option<&WindowGeometry>,
) -> tao::window::WindowBuilder {
    let Some(geometry) = geometry else {
        return builder;
    };

    builder = builder.with_inner_size(PhysicalSize::new(geometry.width, geometry.height));
    if let (Some(x), Some(y)) = (geometry.x, geometry.y) {
        builder = builder.with_position(PhysicalPosition::new(x, y));
    }
    if geometry.maximized {
        builder = builder.with_maximized(true);
    }
    if geometry.fullscreen {
        builder = builder.with_fullscreen(Some(tao::window::Fullscreen::Borderless(None)));
    }
    builder
}

fn read_state_file(path: &Path) -> Result<PersistedWindowState> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(PersistedWindowState::default());
        }
        Err(error) => {
            return Err(error)
                .with_context(|| format!("failed to read window state at {}", path.display()));
        }
    };

    serde_json::from_str(&contents)
        .with_context(|| format!("failed to parse window state at {}", path.display()))
}

fn valid_state(mut state: PersistedWindowState) -> PersistedWindowState {
    state.windows.retain(|persist_key, geometry| {
        let valid = geometry.width > 0 && geometry.height > 0;
        if !valid {
            warn!(persist_key, "ignoring invalid persisted window geometry");
        }
        valid
    });
    state
}

fn state_file_path(paths: &RuntimePaths) -> PathBuf {
    paths.data_dir.join(WINDOW_STATE_FILE)
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use cefari_core::RuntimePaths;

    use super::{
        PersistedWindowState, WindowGeometry, WindowStateStore, persist_key_from_request,
        state_file_path,
    };

    #[test]
    fn saves_and_restores_window_geometry() {
        let fixture = Fixture::new("save-restore");
        let mut store = WindowStateStore::load(&fixture.paths);
        store.state.windows.insert(
            "main".to_owned(),
            WindowGeometry {
                width: 900,
                height: 700,
                x: Some(40),
                y: Some(80),
                maximized: true,
                fullscreen: false,
            },
        );
        store.dirty = true;
        store.flush().expect("state should save");

        let restored = WindowStateStore::load(&fixture.paths);
        assert_eq!(
            restored.geometry("main"),
            Some(&WindowGeometry {
                width: 900,
                height: 700,
                x: Some(40),
                y: Some(80),
                maximized: true,
                fullscreen: false,
            })
        );
    }

    #[test]
    fn missing_file_restores_empty_state() {
        let fixture = Fixture::new("missing");
        let store = WindowStateStore::load(&fixture.paths);

        assert!(store.geometry("main").is_none());
    }

    #[test]
    fn corrupt_file_is_ignored() {
        let fixture = Fixture::new("corrupt");
        fs::create_dir_all(&fixture.paths.data_dir).expect("data dir should be created");
        fs::write(state_file_path(&fixture.paths), "{not json").expect("state should write");

        let store = WindowStateStore::load(&fixture.paths);

        assert!(store.geometry("main").is_none());
    }

    #[test]
    fn invalid_geometry_is_ignored() {
        let fixture = Fixture::new("invalid-geometry");
        fs::create_dir_all(&fixture.paths.data_dir).expect("data dir should be created");
        fs::write(
            state_file_path(&fixture.paths),
            "{\"windows\":{\"main\":{\"width\":0,\"height\":700,\"x\":null,\"y\":null,\"maximized\":false,\"fullscreen\":false}}}",
        )
        .expect("state should write");

        let store = WindowStateStore::load(&fixture.paths);

        assert!(store.geometry("main").is_none());
    }

    #[test]
    fn empty_persist_keys_are_ignored() {
        assert_eq!(persist_key_from_request(None), None);
        assert_eq!(persist_key_from_request(Some("")), None);
        assert_eq!(persist_key_from_request(Some("  ")), None);
        assert_eq!(
            persist_key_from_request(Some("settings")),
            Some("settings".to_owned())
        );
    }

    #[test]
    fn deserializes_empty_state_file() {
        let state: PersistedWindowState =
            serde_json::from_str("{\"windows\":{}}").expect("state should parse");
        assert!(state.windows.is_empty());
    }

    struct Fixture {
        paths: RuntimePaths,
    }

    impl Fixture {
        fn new(name: &str) -> Self {
            let root = std::env::temp_dir().join(format!(
                "cefari-desktop-window-state-{name}-{}",
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&root);
            Self {
                paths: runtime_paths(&root),
            }
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(
                self.paths
                    .data_dir
                    .parent()
                    .expect("data dir should have parent"),
            );
        }
    }

    fn runtime_paths(root: &Path) -> RuntimePaths {
        RuntimePaths {
            config_dir: root.join("config"),
            config_file: root.join("config/cefari.json"),
            data_dir: root.join("data"),
            cache_dir: root.join("cache"),
            log_dir: root.join("data/logs"),
            resource_dir: root.join("data/resources"),
            update_dir: root.join("data/updates"),
        }
    }
}
