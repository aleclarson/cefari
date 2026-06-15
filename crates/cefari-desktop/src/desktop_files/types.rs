use cap_std::{
    fs::FileType,
    time::{SystemClock, SystemTime as CapSystemTime},
};
use cefari_core::FileKind;

pub(super) fn file_kind(file_type: FileType) -> FileKind {
    if file_type.is_file() {
        FileKind::File
    } else if file_type.is_dir() {
        FileKind::Directory
    } else if file_type.is_symlink() {
        FileKind::Symlink
    } else {
        FileKind::Other
    }
}

pub(super) fn system_time_to_ms(time: Option<CapSystemTime>) -> Option<f64> {
    time.and_then(|time| time.duration_since(SystemClock::UNIX_EPOCH).ok())
        .map(|duration| u128_to_js_number(duration.as_millis()))
}

#[expect(
    clippy::cast_precision_loss,
    reason = "IPC exposes JavaScript-compatible number fields instead of BigInt"
)]
pub(super) fn usize_to_js_number(value: usize) -> f64 {
    value as f64
}

#[expect(
    clippy::cast_precision_loss,
    reason = "IPC exposes JavaScript-compatible number fields instead of BigInt"
)]
pub(super) fn u64_to_js_number(value: u64) -> f64 {
    value as f64
}

#[expect(
    clippy::cast_precision_loss,
    reason = "IPC exposes JavaScript-compatible number fields instead of BigInt"
)]
fn u128_to_js_number(value: u128) -> f64 {
    value as f64
}
