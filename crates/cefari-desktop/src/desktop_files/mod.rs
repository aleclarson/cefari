mod ops;
mod paths;
mod types;

use anyhow::{Context, Result};
use cap_std::{ambient_authority, fs_utf8::Dir};
use cefari_core::{AppDataDirInfo, FileResult, FilesCommand, RuntimePaths};

use self::paths::path_to_utf8;

pub(crate) fn checked_app_data_relative_path(path: &str) -> Result<&str> {
    paths::checked_path(path)
}

#[derive(Debug)]
pub struct AppDataFs {
    pub(super) root: Dir,
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
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use cefari_core::{
        FileContents, FileEncoding, FileReadRequest, FileResult, FileWriteOptions,
        FileWriteRequest, FilesCommand, ReadDirRequest, RmRequest, RuntimePaths,
    };

    use super::AppDataFs;

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
