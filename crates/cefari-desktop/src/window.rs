use std::collections::HashMap;

use anyhow::{Context, Result};
use cefari_core::{WindowCreateRequest, WindowKind, WindowState};
use tao::{
    dpi::{LogicalPosition, LogicalSize},
    event_loop::{EventLoop, EventLoopWindowTarget},
    window::{Window, WindowBuilder, WindowId as TaoWindowId},
};
use tracing::error;

use crate::{desktop_ui, event_loop::UserEvent};

const MAIN_WINDOW_TITLE: &str = "Cefari";
const MAIN_WINDOW_WIDTH: f64 = 1200.0;
const MAIN_WINDOW_HEIGHT: f64 = 800.0;
const MIN_WINDOW_WIDTH: f64 = 800.0;
const MIN_WINDOW_HEIGHT: f64 = 560.0;

pub(crate) const MAIN_WINDOW_ID: &str = "main";
const GENERATED_WINDOW_ID_PREFIX: &str = "window";

#[derive(Debug)]
struct WindowRecord {
    id: String,
    kind: WindowKind,
    window: Window,
    title: String,
    route: Option<String>,
    modal: bool,
    parent_id: Option<String>,
}

pub(crate) struct WindowManager {
    main: Option<WindowRecord>,
    secondary: HashMap<String, WindowRecord>,
    tao_windows: HashMap<TaoWindowId, String>,
    next_generated_id: u64,
}

impl WindowManager {
    pub(crate) fn with_main(window: Window) -> Self {
        let tao_id = window.id();
        let main = WindowRecord {
            id: MAIN_WINDOW_ID.to_owned(),
            kind: WindowKind::Main,
            window,
            title: default_window_title(),
            route: None,
            modal: false,
            parent_id: None,
        };
        let mut tao_windows = HashMap::new();
        tao_windows.insert(tao_id, MAIN_WINDOW_ID.to_owned());

        Self {
            main: Some(main),
            secondary: HashMap::new(),
            tao_windows,
            next_generated_id: 1,
        }
    }

    pub(crate) fn window(&self, id: &str) -> Result<&Window> {
        self.record(id)
            .map(|record| &record.window)
            .with_context(|| format!("window {id} is not available"))
    }

    pub(crate) fn window_id_for_tao(&self, tao_id: TaoWindowId) -> Option<String> {
        self.tao_windows.get(&tao_id).cloned()
    }

    pub(crate) fn states(&self) -> Vec<WindowState> {
        self.main
            .iter()
            .chain(self.secondary.values())
            .map(WindowRecord::state)
            .collect()
    }

    pub(crate) fn state(&self, id: &str) -> Result<WindowState> {
        self.record(id)
            .map(WindowRecord::state)
            .with_context(|| format!("window {id} is not available"))
    }

    pub(crate) fn create_secondary(
        &mut self,
        event_loop: &EventLoopWindowTarget<UserEvent>,
        request: &WindowCreateRequest,
    ) -> Result<WindowState> {
        let id = self.resolve_create_id(request.id.as_deref())?;
        let title = request
            .title
            .clone()
            .unwrap_or_else(default_secondary_window_title);
        let route = request.route.clone();
        let modal = request.modal.unwrap_or(false);
        let parent_id = request.parent_id.clone();
        let window = secondary_window_builder(request, &title)
            .build(event_loop)
            .with_context(|| format!("failed to create Cefari window {id}"))?;
        let tao_id = window.id();
        let record = WindowRecord {
            id: id.clone(),
            kind: WindowKind::Secondary,
            window,
            title,
            route,
            modal,
            parent_id,
        };
        let state = record.state();
        self.secondary.insert(id.clone(), record);
        self.tao_windows.insert(tao_id, id);
        Ok(state)
    }

    pub(crate) fn remove_window(&mut self, id: &str) -> Result<WindowState> {
        if id == MAIN_WINDOW_ID {
            return Ok(self.close_main());
        }

        let record = self
            .secondary
            .remove(id)
            .with_context(|| format!("window {id} is not available"))?;
        self.tao_windows.remove(&record.window.id());
        Ok(record.closed_state())
    }

    pub(crate) fn close_main(&mut self) -> WindowState {
        let Some(record) = self.main.take() else {
            return closed_main_state(default_window_title());
        };
        self.tao_windows.remove(&record.window.id());
        self.secondary.clear();
        self.tao_windows.clear();
        record.closed_state()
    }

    pub(crate) fn show_window(&mut self, id: &str) -> Result<WindowState> {
        let record = self
            .record_mut(id)
            .with_context(|| format!("window {id} is not available"))?;
        record.window.set_visible(true);
        Ok(record.state())
    }

    pub(crate) fn focus_window(&mut self, id: &str) -> Result<WindowState> {
        let record = self
            .record_mut(id)
            .with_context(|| format!("window {id} is not available"))?;
        record.window.set_visible(true);
        record.window.set_focus();
        Ok(record.state())
    }

    pub(crate) fn set_window_title(&mut self, id: &str, title: &str) -> Result<WindowState> {
        let record = self
            .record_mut(id)
            .with_context(|| format!("window {id} is not available"))?;
        record.window.set_title(title);
        title.clone_into(&mut record.title);
        Ok(record.state())
    }

    fn record(&self, id: &str) -> Option<&WindowRecord> {
        if id == MAIN_WINDOW_ID {
            self.main.as_ref()
        } else {
            self.secondary.get(id)
        }
    }

    fn record_mut(&mut self, id: &str) -> Option<&mut WindowRecord> {
        if id == MAIN_WINDOW_ID {
            self.main.as_mut()
        } else {
            self.secondary.get_mut(id)
        }
    }

    fn resolve_create_id(&mut self, requested: Option<&str>) -> Result<String> {
        if let Some(id) = requested.filter(|id| !id.is_empty()) {
            validate_window_id(id)?;
            if id == MAIN_WINDOW_ID || self.secondary.contains_key(id) {
                anyhow::bail!("window {id} already exists");
            }
            return Ok(id.to_owned());
        }

        loop {
            let id = format!("{GENERATED_WINDOW_ID_PREFIX}-{}", self.next_generated_id);
            self.next_generated_id += 1;
            if !self.secondary.contains_key(&id) {
                return Ok(id);
            }
        }
    }
}

impl WindowRecord {
    fn state(&self) -> WindowState {
        WindowState {
            id: self.id.clone(),
            kind: self.kind.clone(),
            visible: self.window.is_visible(),
            focused: self.window.is_focused(),
            title: self.title.clone(),
            modal: self.modal,
            parent_id: self.parent_id.clone(),
            route: self.route.clone(),
        }
    }

    fn closed_state(self) -> WindowState {
        WindowState {
            id: self.id,
            kind: self.kind,
            visible: false,
            focused: false,
            title: self.title,
            modal: self.modal,
            parent_id: self.parent_id,
            route: self.route,
        }
    }
}

pub(crate) fn default_window_title() -> String {
    MAIN_WINDOW_TITLE.to_owned()
}

fn default_secondary_window_title() -> String {
    MAIN_WINDOW_TITLE.to_owned()
}

fn closed_main_state(title: String) -> WindowState {
    WindowState {
        id: MAIN_WINDOW_ID.to_owned(),
        kind: WindowKind::Main,
        visible: false,
        focused: false,
        title,
        modal: false,
        parent_id: None,
        route: None,
    }
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

fn secondary_window_builder(request: &WindowCreateRequest, title: &str) -> WindowBuilder {
    let mut builder = WindowBuilder::new()
        .with_title(title)
        .with_inner_size(LogicalSize::new(
            f64::from(request.width.unwrap_or(800)),
            f64::from(request.height.unwrap_or(600)),
        ));

    if let (Some(width), Some(height)) = (request.min_width, request.min_height) {
        builder =
            builder.with_min_inner_size(LogicalSize::new(f64::from(width), f64::from(height)));
    }
    if let (Some(width), Some(height)) = (request.max_width, request.max_height) {
        builder =
            builder.with_max_inner_size(LogicalSize::new(f64::from(width), f64::from(height)));
    }
    if let (Some(x), Some(y)) = (request.x, request.y) {
        builder = builder.with_position(LogicalPosition::new(f64::from(x), f64::from(y)));
    }
    if let Some(visible) = request.visible {
        builder = builder.with_visible(visible);
    }
    if let Some(focused) = request.focused {
        builder = builder.with_focused(focused);
    }
    if let Some(resizable) = request.resizable {
        builder = builder.with_resizable(resizable);
    }
    if let Some(decorations) = request.decorations {
        builder = builder.with_decorations(decorations);
    }
    if let Some(always_on_top) = request.always_on_top {
        builder = builder.with_always_on_top(always_on_top);
    }

    builder
}

pub(crate) fn window_url(base_url: &str, window_id: &str, route: Option<&str>) -> Result<String> {
    validate_window_id(window_id)?;
    let route = route.unwrap_or_default();
    let route = if route.is_empty() { "/" } else { route };
    if !route.starts_with('/') {
        anyhow::bail!("window route must start with /");
    }

    if base_url.starts_with("http://") || base_url.starts_with("https://") {
        return Ok(dev_window_url(base_url, window_id, route));
    }

    Ok(format!(
        "{}://{}/index.html?cefariWindowId={}#{}",
        desktop_ui::CEFARI_APP_SCHEME,
        desktop_ui::CEFARI_APP_HOST,
        window_id,
        route
    ))
}

fn dev_window_url(base_url: &str, window_id: &str, route: &str) -> String {
    let origin_end = base_url
        .find("://")
        .and_then(|scheme_end| {
            let authority_start = scheme_end + 3;
            base_url[authority_start..]
                .find(['/', '?', '#'])
                .map(|offset| authority_start + offset)
        })
        .unwrap_or(base_url.len());
    let origin = &base_url[..origin_end];
    let separator = if route.contains('?') { '&' } else { '?' };
    format!("{origin}{route}{separator}cefariWindowId={window_id}")
}

fn validate_window_id(id: &str) -> Result<()> {
    if id.is_empty()
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        anyhow::bail!("window id must contain only ASCII letters, digits, ., _, -, or :");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        MAIN_WINDOW_HEIGHT, MAIN_WINDOW_ID, MAIN_WINDOW_TITLE, MAIN_WINDOW_WIDTH,
        MIN_WINDOW_HEIGHT, MIN_WINDOW_WIDTH, main_window_builder, window_url,
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

    #[test]
    fn dev_window_urls_resolve_routes_against_frontend_origin() {
        assert_eq!(
            window_url(
                "http://127.0.0.1:5173/dashboard",
                "settings",
                Some("/settings")
            )
            .expect("URL should resolve"),
            "http://127.0.0.1:5173/settings?cefariWindowId=settings"
        );
    }

    #[test]
    fn packaged_window_urls_use_hash_route_metadata() {
        assert_eq!(
            window_url("cefari://app/index.html", "settings", Some("/settings"))
                .expect("URL should resolve"),
            "cefari://app/index.html?cefariWindowId=settings#/settings"
        );
    }

    #[test]
    fn window_url_rejects_invalid_ids_and_routes() {
        assert!(window_url("cefari://app/index.html", "bad id", Some("/settings")).is_err());
        assert!(window_url("cefari://app/index.html", "settings", Some("settings")).is_err());
    }
}
