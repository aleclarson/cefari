use std::{cell::RefCell, collections::HashMap, path::PathBuf, rc::Rc};

use anyhow::{Context, Result};
use cef::ImplDownloadItemCallback as _;
use cefari_core::{
    CefariIpcEvent, DownloadCanceledEvent, DownloadCompletedEvent, DownloadEvent,
    DownloadFailedEvent, DownloadIdResult, DownloadProgressEvent, DownloadResult,
    DownloadStartedEvent,
};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum DownloadDecision {
    Allow,
    Deny(&'static str),
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct DownloadPolicy;

impl DownloadPolicy {
    pub(crate) fn decide(url: &str) -> DownloadDecision {
        if url.starts_with("https://") || url.starts_with("http://") {
            DownloadDecision::Allow
        } else {
            DownloadDecision::Deny("unsupported download URL")
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct DownloadSnapshot {
    pub(crate) id: String,
    pub(crate) url: String,
    pub(crate) suggested_name: String,
    pub(crate) destination_path: Option<String>,
    pub(crate) received_bytes: i64,
    pub(crate) total_bytes: Option<i64>,
    pub(crate) percent_complete: Option<i32>,
    pub(crate) is_complete: bool,
    pub(crate) is_canceled: bool,
    pub(crate) is_interrupted: bool,
    pub(crate) interrupt_reason: String,
}

#[derive(Clone, Default)]
pub(crate) struct SharedDownloadState(Rc<RefCell<DownloadState>>);

impl SharedDownloadState {
    pub(crate) fn start(&self, snapshot: &DownloadSnapshot) -> CefariIpcEvent {
        let event = DownloadEvent::Started(DownloadStartedEvent {
            id: snapshot.id.clone(),
            url: snapshot.url.clone(),
            suggested_name: snapshot.suggested_name.clone(),
            destination_path: snapshot.destination_path.clone(),
            total_bytes: snapshot.total_bytes.map(download_bytes),
        });
        self.0.borrow_mut().upsert(snapshot);
        CefariIpcEvent::Download(event)
    }

    pub(crate) fn update(
        &self,
        snapshot: &DownloadSnapshot,
        callback: Option<cef::DownloadItemCallback>,
    ) -> Option<CefariIpcEvent> {
        let event = {
            let mut state = self.0.borrow_mut();
            let record = state.upsert(snapshot);
            if let Some(callback) = callback {
                record.callback = Some(callback);
            }

            if snapshot.is_complete {
                record.terminal = true;
                Some(DownloadEvent::Completed(DownloadCompletedEvent {
                    id: snapshot.id.clone(),
                    url: snapshot.url.clone(),
                    destination_path: snapshot.destination_path.clone().unwrap_or_default(),
                    received_bytes: download_bytes(snapshot.received_bytes),
                    total_bytes: snapshot.total_bytes.map(download_bytes),
                }))
            } else if snapshot.is_canceled {
                record.terminal = true;
                Some(DownloadEvent::Canceled(DownloadCanceledEvent {
                    id: snapshot.id.clone(),
                    reason: "canceled".to_owned(),
                }))
            } else if snapshot.is_interrupted {
                record.terminal = true;
                Some(DownloadEvent::Failed(DownloadFailedEvent {
                    id: snapshot.id.clone(),
                    reason: snapshot.interrupt_reason.clone(),
                }))
            } else if snapshot.received_bytes > record.last_emitted_received_bytes {
                record.last_emitted_received_bytes = snapshot.received_bytes;
                Some(DownloadEvent::Progress(DownloadProgressEvent {
                    id: snapshot.id.clone(),
                    received_bytes: download_bytes(snapshot.received_bytes),
                    total_bytes: snapshot.total_bytes.map(download_bytes),
                    percent_complete: snapshot.percent_complete,
                }))
            } else {
                None
            }
        };

        event.map(CefariIpcEvent::Download)
    }

    pub(crate) fn cancel(&self, id: &str) -> Result<DownloadResult> {
        let callback = self
            .0
            .borrow()
            .downloads
            .get(id)
            .and_then(|download| download.callback.clone())
            .with_context(|| format!("active download not found: {id}"))?;
        callback.cancel();
        Ok(DownloadResult::Canceled(DownloadIdResult {
            id: id.to_owned(),
        }))
    }

    pub(crate) fn reveal_path(&self, id: &str) -> Result<PathBuf> {
        let path = self
            .0
            .borrow()
            .downloads
            .get(id)
            .and_then(|download| download.destination_path.clone())
            .map(PathBuf::from)
            .with_context(|| format!("download path not found: {id}"))?;
        if !path.exists() {
            anyhow::bail!("download path does not exist: {}", path.display());
        }
        Ok(path)
    }
}

#[derive(Default)]
struct DownloadState {
    downloads: HashMap<String, DownloadRecord>,
}

impl DownloadState {
    fn upsert(&mut self, snapshot: &DownloadSnapshot) -> &mut DownloadRecord {
        let id = snapshot.id.clone();
        let record = self
            .downloads
            .entry(id)
            .or_insert_with(|| DownloadRecord::new(snapshot));
        record.url.clone_from(&snapshot.url);
        record.suggested_name.clone_from(&snapshot.suggested_name);
        record
            .destination_path
            .clone_from(&snapshot.destination_path);
        record.total_bytes = snapshot.total_bytes;
        record
    }
}

struct DownloadRecord {
    url: String,
    suggested_name: String,
    destination_path: Option<String>,
    total_bytes: Option<i64>,
    callback: Option<cef::DownloadItemCallback>,
    terminal: bool,
    last_emitted_received_bytes: i64,
}

impl DownloadRecord {
    fn new(snapshot: &DownloadSnapshot) -> Self {
        Self {
            url: snapshot.url.clone(),
            suggested_name: snapshot.suggested_name.clone(),
            destination_path: snapshot.destination_path.clone(),
            total_bytes: snapshot.total_bytes,
            callback: None,
            terminal: false,
            last_emitted_received_bytes: 0,
        }
    }
}

#[allow(clippy::cast_precision_loss)]
fn download_bytes(bytes: i64) -> f64 {
    bytes as f64
}

#[cfg(test)]
mod tests {
    use cefari_core::CefariIpcEvent;

    use super::{DownloadDecision, DownloadPolicy, DownloadSnapshot, SharedDownloadState};

    fn snapshot(id: &str, received_bytes: i64) -> DownloadSnapshot {
        DownloadSnapshot {
            id: id.to_owned(),
            url: "https://example.test/file.txt".to_owned(),
            suggested_name: "file.txt".to_owned(),
            destination_path: Some("/tmp/file.txt".to_owned()),
            received_bytes,
            total_bytes: Some(100),
            percent_complete: i32::try_from(received_bytes).ok(),
            is_complete: false,
            is_canceled: false,
            is_interrupted: false,
            interrupt_reason: String::new(),
        }
    }

    #[test]
    fn download_policy_allows_http_and_https_only() {
        assert_eq!(
            DownloadPolicy::decide("https://example.test/file"),
            DownloadDecision::Allow
        );
        assert_eq!(
            DownloadPolicy::decide("http://example.test/file"),
            DownloadDecision::Allow
        );
        assert!(matches!(
            DownloadPolicy::decide("file:///tmp/file"),
            DownloadDecision::Deny(_)
        ));
        assert!(matches!(
            DownloadPolicy::decide("blob:https://example.test/id"),
            DownloadDecision::Deny(_)
        ));
    }

    #[test]
    fn download_state_emits_progress_and_completion() {
        let state = SharedDownloadState::default();
        assert!(matches!(
            state.start(&snapshot("download-1", 0)),
            CefariIpcEvent::Download(_)
        ));
        assert!(matches!(
            state.update(&snapshot("download-1", 50), None),
            Some(CefariIpcEvent::Download(_))
        ));

        let mut completed = snapshot("download-1", 100);
        completed.is_complete = true;
        assert!(matches!(
            state.update(&completed, None),
            Some(CefariIpcEvent::Download(_))
        ));
    }
}
