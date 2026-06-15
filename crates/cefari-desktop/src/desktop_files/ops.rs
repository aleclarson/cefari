use std::{
    io::{self, Write},
    path::Path,
};

use anyhow::{Context, Result};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use cap_std::fs_utf8::OpenOptions;
use cefari_core::{
    CopyFileRequest, DirEntry, FileContents, FileEncoding, FilePathRequest, FileReadRequest,
    FileResult, FileStat, FileWriteRequest, FileWriteResult, MkdirRequest, ReadDirRequest,
    RenameRequest, RmRequest,
};

use super::{
    AppDataFs,
    paths::{checked_path, child_path, temporary_sibling_path},
    types::{file_kind, system_time_to_ms, u64_to_js_number, usize_to_js_number},
};

impl AppDataFs {
    pub(super) fn read_file(&self, request: &FileReadRequest) -> Result<FileResult> {
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

    pub(super) fn write_file(&self, request: &FileWriteRequest) -> Result<FileResult> {
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

    pub(super) fn readdir(&self, request: &ReadDirRequest) -> Result<FileResult> {
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

    pub(super) fn mkdir(&self, request: &MkdirRequest) -> Result<FileResult> {
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

    pub(super) fn rm(&self, request: &RmRequest) -> Result<FileResult> {
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

    pub(super) fn rename(&self, request: &RenameRequest) -> Result<FileResult> {
        let from = checked_path(&request.from)?;
        let to = checked_path(&request.to)?;
        self.create_parent_dirs(to)?;
        self.root
            .rename(from, &self.root, to)
            .with_context(|| format!("failed to rename {from} to {to}"))?;
        Ok(FileResult::Empty)
    }

    pub(super) fn copy_file(&self, request: &CopyFileRequest) -> Result<FileResult> {
        let from = checked_path(&request.from)?;
        let to = checked_path(&request.to)?;
        self.create_parent_dirs(to)?;
        self.root
            .copy(from, &self.root, to)
            .with_context(|| format!("failed to copy file {from} to {to}"))?;
        Ok(FileResult::Empty)
    }

    pub(super) fn stat(&self, request: &FilePathRequest) -> Result<FileResult> {
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

    pub(super) fn access(&self, request: &FilePathRequest) -> Result<bool> {
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
