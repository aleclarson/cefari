use std::{
    io::{self, Write},
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, anyhow};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use cap_std::{
    ambient_authority,
    fs::FileType,
    fs_utf8::{Dir, OpenOptions},
    time::{SystemClock, SystemTime as CapSystemTime},
};
use cefari_core::{
    AppDataDirInfo, CopyFileRequest, DirEntry, FileContents, FileEncoding, FileKind,
    FilePathRequest, FileReadRequest, FileResult, FileStat, FileWriteRequest, FileWriteResult,
    FilesCommand, MkdirRequest, ReadDirRequest, RenameRequest, RmRequest, RuntimePaths,
};

#[derive(Debug)]
pub struct AppDataFs {
    root: Dir,
    display_path: String,
}

impl AppDataFs {
    pub fn open(paths: &RuntimePaths) -> Result<Self> {
        std::fs::create_dir_all(&paths.data_dir).with_context(|| {
            format!(
                "failed to create app data directory at {}",
                paths.data_dir.display()
            )
        })?;

        let root_path = path_to_utf8(&paths.data_dir)?;
        let root = Dir::open_ambient_dir(root_path, ambient_authority()).with_context(|| {
            format!(
                "failed to open app data directory at {}",
                paths.data_dir.display()
            )
        })?;

        Ok(Self {
            root,
            display_path: paths.data_dir.display().to_string(),
        })
    }

    pub fn dispatch(&self, command: &FilesCommand) -> Result<FileResult> {
        match command {
            FilesCommand::AppDataDir => Ok(FileResult::AppDataDir(AppDataDirInfo {
                root_kind: "appData".to_owned(),
                display_path: self.display_path.clone(),
            })),
            FilesCommand::ReadFile(request) => self.read_file(request),
            FilesCommand::WriteFile(request) => self.write_file(request),
            FilesCommand::Readdir(request) => self.readdir(request),
            FilesCommand::Mkdir(request) => self.mkdir(request),
            FilesCommand::Rm(request) => self.rm(request),
            FilesCommand::Rename(request) => self.rename(request),
            FilesCommand::CopyFile(request) => self.copy_file(request),
            FilesCommand::Stat(request) => self.stat(request),
            FilesCommand::Access(request) => Ok(FileResult::Access {
                ok: self.access(request)?,
            }),
        }
    }

    fn read_file(&self, request: &FileReadRequest) -> Result<FileResult> {
        let path = checked_path(&request.path)?;
        match request.encoding.unwrap_or(FileEncoding::Base64) {
            FileEncoding::Utf8 => Ok(FileResult::Text {
                contents: self
                    .root
                    .read_to_string(path)
                    .with_context(|| format!("failed to read file {path}"))?,
            }),
            FileEncoding::Base64 => Ok(FileResult::Base64 {
                contents: BASE64.encode(
                    self.root
                        .read(path)
                        .with_context(|| format!("failed to read file {path}"))?,
                ),
            }),
        }
    }

    fn write_file(&self, request: &FileWriteRequest) -> Result<FileResult> {
        let path = checked_path(&request.path)?;
        if path == "." {
            anyhow::bail!("cannot write app data root as a file");
        }

        let contents = match &request.contents {
            FileContents::Text(text) => text.as_bytes().to_vec(),
            FileContents::Base64(encoded) => BASE64
                .decode(encoded)
                .with_context(|| format!("failed to decode base64 contents for {path}"))?,
        };

        if request.options.create_parents {
            self.create_parent_dirs(path)?;
        }

        if request.options.overwrite {
            self.write_file_atomically(path, &contents)?;
        } else {
            self.write_file_once(path, &contents)?;
        }

        Ok(FileResult::Written(FileWriteResult {
            path: path.to_owned(),
            bytes_written: usize_to_js_number(contents.len()),
        }))
    }

    fn readdir(&self, request: &ReadDirRequest) -> Result<FileResult> {
        let path = checked_path(&request.path)?;
        let mut entries = Vec::new();

        for entry in self
            .root
            .read_dir(path)
            .with_context(|| format!("failed to read directory {path}"))?
        {
            let entry = entry.with_context(|| format!("failed to read entry in {path}"))?;
            let name = entry
                .file_name()
                .with_context(|| format!("failed to read file name in {path}"))?;
            let child_path = child_path(path, &name);
            let kind = file_kind(entry.file_type().with_context(|| {
                format!("failed to read file type for directory entry {child_path}")
            })?);

            if request.with_file_types {
                entries.push(DirEntry {
                    name: name.clone(),
                    path: child_path,
                    kind,
                });
            } else {
                entries.push(DirEntry {
                    name: name.clone(),
                    path: name,
                    kind,
                });
            }
        }

        entries.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(FileResult::DirEntries { entries })
    }

    fn mkdir(&self, request: &MkdirRequest) -> Result<FileResult> {
        let path = checked_path(&request.path)?;
        if request.recursive {
            self.root
                .create_dir_all(path)
                .with_context(|| format!("failed to create directory {path}"))?;
        } else {
            self.root
                .create_dir(path)
                .with_context(|| format!("failed to create directory {path}"))?;
        }
        Ok(FileResult::Empty)
    }

    fn rm(&self, request: &RmRequest) -> Result<FileResult> {
        let path = checked_path(&request.path)?;
        if path == "." {
            anyhow::bail!("cannot remove app data root");
        }

        let metadata = match self.root.symlink_metadata(path) {
            Ok(metadata) => metadata,
            Err(error) if request.force && error.kind() == io::ErrorKind::NotFound => {
                return Ok(FileResult::Empty);
            }
            Err(error) => return Err(error).with_context(|| format!("failed to inspect {path}")),
        };

        if metadata.is_dir() {
            if request.recursive {
                self.root
                    .remove_dir_all(path)
                    .with_context(|| format!("failed to recursively remove directory {path}"))?;
            } else {
                self.root
                    .remove_dir(path)
                    .with_context(|| format!("failed to remove directory {path}"))?;
            }
        } else {
            self.root
                .remove_file(path)
                .with_context(|| format!("failed to remove file {path}"))?;
        }

        Ok(FileResult::Empty)
    }

    fn rename(&self, request: &RenameRequest) -> Result<FileResult> {
        let from = checked_path(&request.from)?;
        let to = checked_path(&request.to)?;
        self.create_parent_dirs(to)?;
        self.root
            .rename(from, &self.root, to)
            .with_context(|| format!("failed to rename {from} to {to}"))?;
        Ok(FileResult::Empty)
    }

    fn copy_file(&self, request: &CopyFileRequest) -> Result<FileResult> {
        let from = checked_path(&request.from)?;
        let to = checked_path(&request.to)?;
        self.create_parent_dirs(to)?;
        self.root
            .copy(from, &self.root, to)
            .with_context(|| format!("failed to copy file {from} to {to}"))?;
        Ok(FileResult::Empty)
    }

    fn stat(&self, request: &FilePathRequest) -> Result<FileResult> {
        let path = checked_path(&request.path)?;
        let metadata = self
            .root
            .symlink_metadata(path)
            .with_context(|| format!("failed to stat {path}"))?;

        Ok(FileResult::Stat(FileStat {
            path: path.to_owned(),
            kind: file_kind(metadata.file_type()),
            size: u64_to_js_number(metadata.len()),
            modified_at_ms: system_time_to_ms(metadata.modified().ok()),
            created_at_ms: system_time_to_ms(metadata.created().ok()),
        }))
    }

    fn access(&self, request: &FilePathRequest) -> Result<bool> {
        let path = checked_path(&request.path)?;
        match self.root.symlink_metadata(path) {
            Ok(_) => Ok(true),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(error).with_context(|| format!("failed to access {path}")),
        }
    }

    fn write_file_atomically(&self, path: &str, contents: &[u8]) -> Result<()> {
        let temporary_path = temporary_sibling_path(path)?;
        let write_result = self.write_file_once(&temporary_path, contents);

        if let Err(error) = write_result {
            let _ = self.root.remove_file(&temporary_path);
            return Err(error);
        }

        self.root
            .rename(&temporary_path, &self.root, path)
            .with_context(|| format!("failed to replace {path}"))?;
        Ok(())
    }

    fn write_file_once(&self, path: &str, contents: &[u8]) -> Result<()> {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        let mut file = self
            .root
            .open_with(path, &options)
            .with_context(|| format!("failed to create file {path}"))?;
        file.write_all(contents)
            .with_context(|| format!("failed to write file {path}"))
    }

    fn create_parent_dirs(&self, path: &str) -> Result<()> {
        let Some(parent) = Path::new(path).parent() else {
            return Ok(());
        };
        let parent = parent.to_string_lossy();
        if parent.is_empty() {
            return Ok(());
        }
        self.root
            .create_dir_all(parent.as_ref())
            .with_context(|| format!("failed to create parent directory {parent}"))
    }
}

fn path_to_utf8(path: &Path) -> Result<&str> {
    path.to_str()
        .ok_or_else(|| anyhow!("path is not valid UTF-8: {}", path.display()))
}

fn checked_path(path: &str) -> Result<&str> {
    if path.is_empty() || path == "." {
        return Ok(".");
    }

    let path = Path::new(path);
    if path.is_absolute() {
        anyhow::bail!("absolute paths are not allowed");
    }

    for component in path.components() {
        match component {
            std::path::Component::Normal(_) | std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                anyhow::bail!("parent path traversal is not allowed")
            }
            std::path::Component::Prefix(_) | std::path::Component::RootDir => {
                anyhow::bail!("absolute paths are not allowed");
            }
        }
    }

    path_to_utf8(path)
}

fn child_path(parent: &str, name: &str) -> String {
    if parent == "." {
        name.to_owned()
    } else {
        format!("{parent}/{name}")
    }
}

fn temporary_sibling_path(path: &str) -> Result<String> {
    let path = Path::new(path);
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow!("file path must end with a UTF-8 file name"))?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before Unix epoch")?
        .as_nanos();
    let temporary_name = format!(".{file_name}.cefari-tmp-{nonce}");

    Ok(path.parent().map_or_else(
        || temporary_name.clone(),
        |parent| {
            if parent.as_os_str().is_empty() {
                temporary_name.clone()
            } else {
                parent.join(&temporary_name).to_string_lossy().into_owned()
            }
        },
    ))
}

fn file_kind(file_type: FileType) -> FileKind {
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

fn system_time_to_ms(time: Option<CapSystemTime>) -> Option<f64> {
    time.and_then(|time| time.duration_since(SystemClock::UNIX_EPOCH).ok())
        .map(|duration| u128_to_js_number(duration.as_millis()))
}

#[expect(
    clippy::cast_precision_loss,
    reason = "IPC exposes JavaScript-compatible number fields instead of BigInt"
)]
fn usize_to_js_number(value: usize) -> f64 {
    value as f64
}

#[expect(
    clippy::cast_precision_loss,
    reason = "IPC exposes JavaScript-compatible number fields instead of BigInt"
)]
fn u64_to_js_number(value: u64) -> f64 {
    value as f64
}

#[expect(
    clippy::cast_precision_loss,
    reason = "IPC exposes JavaScript-compatible number fields instead of BigInt"
)]
fn u128_to_js_number(value: u128) -> f64 {
    value as f64
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use cefari_core::{
        FileContents, FileEncoding, FileReadRequest, FileResult, FileWriteOptions,
        FileWriteRequest, FilesCommand, ReadDirRequest, RmRequest,
    };

    use super::{AppDataFs, RuntimePaths};

    #[test]
    fn reads_and_writes_text_inside_app_data_root() {
        let fixture = Fixture::new("text_round_trip");
        let fs = AppDataFs::open(&fixture.paths).expect("filesystem should open");

        let written = fs
            .dispatch(&FilesCommand::WriteFile(FileWriteRequest {
                path: "settings/state.json".to_owned(),
                contents: FileContents::Text("{\"ok\":true}".to_owned()),
                options: FileWriteOptions {
                    create_parents: true,
                    overwrite: true,
                },
            }))
            .expect("file should write");
        assert!(matches!(written, FileResult::Written(_)));

        let read = fs
            .dispatch(&FilesCommand::ReadFile(FileReadRequest {
                path: "settings/state.json".to_owned(),
                encoding: Some(FileEncoding::Utf8),
            }))
            .expect("file should read");
        assert_eq!(
            read,
            FileResult::Text {
                contents: "{\"ok\":true}".to_owned()
            }
        );
    }

    #[test]
    fn rejects_parent_traversal_before_capability_access() {
        let fixture = Fixture::new("rejects_traversal");
        let fs = AppDataFs::open(&fixture.paths).expect("filesystem should open");

        let error = fs
            .dispatch(&FilesCommand::ReadFile(FileReadRequest {
                path: "../secret.txt".to_owned(),
                encoding: Some(FileEncoding::Utf8),
            }))
            .expect_err("traversal should fail");

        assert!(error.to_string().contains("parent path traversal"));
    }

    #[test]
    fn lists_and_removes_files() {
        let fixture = Fixture::new("lists_and_removes");
        let fs = AppDataFs::open(&fixture.paths).expect("filesystem should open");

        fs.dispatch(&FilesCommand::WriteFile(FileWriteRequest {
            path: "notes/today.txt".to_owned(),
            contents: FileContents::Text("done".to_owned()),
            options: FileWriteOptions {
                create_parents: true,
                overwrite: true,
            },
        }))
        .expect("file should write");

        let entries = fs
            .dispatch(&FilesCommand::Readdir(ReadDirRequest {
                path: "notes".to_owned(),
                with_file_types: true,
            }))
            .expect("directory should read");

        match entries {
            FileResult::DirEntries { entries } => {
                assert_eq!(entries.len(), 1);
                assert_eq!(entries[0].path, "notes/today.txt");
            }
            other => panic!("unexpected file result: {other:?}"),
        }

        fs.dispatch(&FilesCommand::Rm(RmRequest {
            path: "notes".to_owned(),
            recursive: true,
            force: false,
        }))
        .expect("directory should remove");
        assert!(!fixture.paths.data_dir.join("notes").exists());
    }

    struct Fixture {
        paths: RuntimePaths,
    }

    impl Fixture {
        fn new(name: &str) -> Self {
            let root = std::env::temp_dir().join(format!(
                "cefari-desktop-files-{name}-{}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&root);
            let paths = RuntimePaths {
                config_dir: root.join("config"),
                config_file: root.join("config").join("cefari.json"),
                data_dir: root.join("data"),
                cache_dir: root.join("cache"),
                log_dir: root.join("data").join("logs"),
                resource_dir: root.join("data").join("resources"),
                update_dir: root.join("data").join("updates"),
            };
            Self { paths }
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let root = self
                .paths
                .data_dir
                .parent()
                .map(PathBuf::from)
                .expect("fixture data directory should have a parent");
            let _ = std::fs::remove_dir_all(root);
        }
    }
}
