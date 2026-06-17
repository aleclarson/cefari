use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use cefari_core::{
    DialogCommand, DialogDefaultDirectory, DialogFilter, DialogModality, DialogRequest,
    DialogResult, DialogSelectedPath, FileKind, RuntimePaths,
};
use rfd::FileDialog;
use tao::window::Window;

use crate::desktop_files;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum DialogOperation {
    OpenFile,
    OpenFiles,
    ChooseFolder,
    ChooseFolders,
    SaveFile,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct DialogInvocation {
    operation: DialogOperation,
    title: Option<String>,
    filters: Vec<DialogFilter>,
    default_directory: Option<PathBuf>,
    default_name: Option<String>,
    parented: bool,
    can_create_directories: Option<bool>,
}

trait NativeDialogProvider {
    fn show(
        &self,
        invocation: &DialogInvocation,
        parent: Option<&Window>,
    ) -> Result<Option<Vec<PathBuf>>>;
}

#[derive(Debug, Default)]
struct RfdDialogProvider;

pub(crate) fn dispatch(
    command: &DialogCommand,
    paths: &RuntimePaths,
    parent: Option<&Window>,
) -> Result<DialogResult> {
    dispatch_with_provider(command, paths, parent, &RfdDialogProvider)
}

fn dispatch_with_provider(
    command: &DialogCommand,
    paths: &RuntimePaths,
    parent: Option<&Window>,
    provider: &impl NativeDialogProvider,
) -> Result<DialogResult> {
    let invocation = invocation_for_command(command, paths)?;
    let Some(paths) = provider.show(&invocation, parent)? else {
        return Ok(DialogResult::Canceled);
    };
    Ok(DialogResult::Selected {
        paths: selected_paths(invocation.operation, paths)?,
    })
}

impl NativeDialogProvider for RfdDialogProvider {
    fn show(
        &self,
        invocation: &DialogInvocation,
        parent: Option<&Window>,
    ) -> Result<Option<Vec<PathBuf>>> {
        let dialog = rfd_dialog(invocation, parent);
        let paths = match invocation.operation {
            DialogOperation::OpenFile => dialog.pick_file().map(|path| vec![path]),
            DialogOperation::OpenFiles => dialog.pick_files(),
            DialogOperation::ChooseFolder => dialog.pick_folder().map(|path| vec![path]),
            DialogOperation::ChooseFolders => dialog.pick_folders(),
            DialogOperation::SaveFile => dialog.save_file().map(|path| vec![path]),
        };
        Ok(paths)
    }
}

fn rfd_dialog(invocation: &DialogInvocation, parent: Option<&Window>) -> FileDialog {
    let mut dialog = FileDialog::new();

    for filter in &invocation.filters {
        let extensions = filter
            .extensions
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        dialog = dialog.add_filter(&filter.name, &extensions);
    }

    if let Some(title) = &invocation.title {
        dialog = dialog.set_title(title);
    }
    if let Some(default_directory) = &invocation.default_directory {
        dialog = dialog.set_directory(default_directory);
    }
    if let Some(default_name) = &invocation.default_name {
        dialog = dialog.set_file_name(default_name);
    }
    if let Some(can_create_directories) = invocation.can_create_directories {
        dialog = dialog.set_can_create_directories(can_create_directories);
    }
    if invocation.parented
        && let Some(parent) = parent
    {
        dialog = dialog.set_parent(parent);
    }

    dialog
}

fn invocation_for_command(
    command: &DialogCommand,
    paths: &RuntimePaths,
) -> Result<DialogInvocation> {
    let (operation, request) = match command {
        DialogCommand::OpenFile(request) => (DialogOperation::OpenFile, request),
        DialogCommand::OpenFiles(request) => (DialogOperation::OpenFiles, request),
        DialogCommand::ChooseFolder(request) => (DialogOperation::ChooseFolder, request),
        DialogCommand::ChooseFolders(request) => (DialogOperation::ChooseFolders, request),
        DialogCommand::SaveFile(request) => (DialogOperation::SaveFile, request),
    };

    Ok(DialogInvocation {
        operation,
        title: optional_nonblank_text(request.title.as_deref(), "dialog title")?,
        filters: validate_filters(&request.filters)?,
        default_directory: resolve_default_directory(request, paths)?,
        default_name: optional_file_name(request.default_name.as_deref())?,
        parented: request.modality.unwrap_or(DialogModality::Window) == DialogModality::Window,
        can_create_directories: request.can_create_directories,
    })
}

fn validate_filters(filters: &[DialogFilter]) -> Result<Vec<DialogFilter>> {
    filters
        .iter()
        .map(|filter| {
            let name = optional_nonblank_text(Some(&filter.name), "dialog filter name")?
                .ok_or_else(|| anyhow!("dialog filter name is required"))?;
            if filter.extensions.is_empty() {
                anyhow::bail!("dialog filter {name} must include at least one extension");
            }
            let extensions = filter
                .extensions
                .iter()
                .map(|extension| validate_extension(extension, &name))
                .collect::<Result<Vec<_>>>()?;

            Ok(DialogFilter { name, extensions })
        })
        .collect()
}

fn validate_extension(extension: &str, filter_name: &str) -> Result<String> {
    let extension = optional_nonblank_text(Some(extension), "dialog filter extension")?
        .ok_or_else(|| anyhow!("dialog filter extension is required"))?;
    if extension.starts_with('.') {
        anyhow::bail!("dialog filter {filter_name} extension must not start with a dot");
    }
    if extension.contains('*') || extension.contains('?') {
        anyhow::bail!("dialog filter {filter_name} extension must not be a glob");
    }
    if extension.contains('/') || extension.contains('\\') {
        anyhow::bail!("dialog filter {filter_name} extension must not contain path separators");
    }
    Ok(extension)
}

fn optional_nonblank_text(value: Option<&str>, label: &str) -> Result<Option<String>> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.contains('\0') {
        anyhow::bail!("{label} must not contain NUL bytes");
    }
    let trimmed = value.trim();
    if trimmed.is_empty() {
        anyhow::bail!("{label} must not be blank");
    }
    Ok(Some(trimmed.to_owned()))
}

fn optional_file_name(value: Option<&str>) -> Result<Option<String>> {
    let Some(name) = optional_nonblank_text(value, "dialog default name")? else {
        return Ok(None);
    };
    if name.contains('/') || name.contains('\\') {
        anyhow::bail!("dialog default name must not contain path separators");
    }
    Ok(Some(name))
}

fn resolve_default_directory(
    request: &DialogRequest,
    paths: &RuntimePaths,
) -> Result<Option<PathBuf>> {
    match &request.default_directory {
        None => Ok(None),
        Some(DialogDefaultDirectory::Native { path }) => {
            let path = optional_nonblank_text(Some(path), "dialog default directory")?
                .ok_or_else(|| anyhow!("dialog default directory is required"))?;
            Ok(Some(PathBuf::from(path)))
        }
        Some(DialogDefaultDirectory::AppData { path }) => {
            let relative =
                desktop_files::checked_app_data_relative_path(path.as_deref().unwrap_or("."))?;
            Ok(Some(if relative == "." {
                paths.data_dir.clone()
            } else {
                paths.data_dir.join(relative)
            }))
        }
    }
}

fn selected_paths(
    operation: DialogOperation,
    paths: Vec<PathBuf>,
) -> Result<Vec<DialogSelectedPath>> {
    paths
        .into_iter()
        .map(|path| selected_path(operation, &path))
        .collect()
}

fn selected_path(operation: DialogOperation, path: &Path) -> Result<DialogSelectedPath> {
    let path_text = path.to_str().ok_or_else(|| {
        anyhow!(
            "selected dialog path is not valid UTF-8: {}",
            path.display()
        )
    })?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow!("selected dialog path does not end with a UTF-8 file name"))?;

    Ok(DialogSelectedPath {
        path: path_text.to_owned(),
        name: name.to_owned(),
        kind: selected_kind(operation, path),
    })
}

fn selected_kind(operation: DialogOperation, path: &Path) -> FileKind {
    if operation == DialogOperation::SaveFile && !path.exists() {
        return FileKind::File;
    }
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => FileKind::Symlink,
        Ok(metadata) if metadata.is_file() => FileKind::File,
        Ok(metadata) if metadata.is_dir() => FileKind::Directory,
        Ok(_) | Err(_) => {
            if matches!(
                operation,
                DialogOperation::ChooseFolder | DialogOperation::ChooseFolders
            ) {
                FileKind::Directory
            } else {
                FileKind::Other
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DialogInvocation, DialogOperation, NativeDialogProvider, dispatch_with_provider,
        invocation_for_command,
    };
    use anyhow::Result;
    use cefari_core::{
        DialogCommand, DialogDefaultDirectory, DialogFilter, DialogModality, DialogRequest,
        DialogResult, FileKind, RuntimePaths,
    };
    use std::{
        cell::RefCell,
        path::{MAIN_SEPARATOR_STR, PathBuf},
    };
    use tao::window::Window;

    #[derive(Debug)]
    struct FakeDialogProvider {
        response: Option<Vec<PathBuf>>,
        invocations: RefCell<Vec<DialogInvocation>>,
    }

    impl NativeDialogProvider for FakeDialogProvider {
        fn show(
            &self,
            invocation: &DialogInvocation,
            _parent: Option<&Window>,
        ) -> Result<Option<Vec<PathBuf>>> {
            self.invocations.borrow_mut().push(invocation.clone());
            Ok(self.response.clone())
        }
    }

    #[test]
    fn maps_each_dialog_operation_to_provider_invocation() {
        let paths = fixture_paths("maps_each_dialog_operation");
        let request = request();
        let commands = [
            (
                DialogCommand::OpenFile(request.clone()),
                DialogOperation::OpenFile,
            ),
            (
                DialogCommand::OpenFiles(request.clone()),
                DialogOperation::OpenFiles,
            ),
            (
                DialogCommand::ChooseFolder(request.clone()),
                DialogOperation::ChooseFolder,
            ),
            (
                DialogCommand::ChooseFolders(request.clone()),
                DialogOperation::ChooseFolders,
            ),
            (DialogCommand::SaveFile(request), DialogOperation::SaveFile),
        ];

        for (command, operation) in commands {
            let invocation =
                invocation_for_command(&command, &paths).expect("dialog command should normalize");

            assert_eq!(invocation.operation, operation);
            assert_eq!(invocation.title.as_deref(), Some("Choose File"));
            assert_eq!(invocation.default_name.as_deref(), Some("report.txt"));
            assert!(invocation.parented);
            assert_eq!(invocation.can_create_directories, Some(true));
        }
    }

    #[test]
    fn returns_canceled_when_provider_has_no_selection() {
        let paths = fixture_paths("returns_canceled");
        let provider = FakeDialogProvider {
            response: None,
            invocations: RefCell::new(Vec::new()),
        };

        let result =
            dispatch_with_provider(&DialogCommand::OpenFile(request()), &paths, None, &provider)
                .expect("dialog should dispatch");

        assert_eq!(result, DialogResult::Canceled);
    }

    #[test]
    fn returns_selected_paths_with_metadata() {
        let paths = fixture_paths("returns_selected_paths");
        std::fs::create_dir_all(&paths.data_dir).expect("data dir should create");
        let selected = paths.data_dir.join("report.txt");
        std::fs::write(&selected, "done").expect("selected file should write");
        let provider = FakeDialogProvider {
            response: Some(vec![selected.clone()]),
            invocations: RefCell::new(Vec::new()),
        };

        let result =
            dispatch_with_provider(&DialogCommand::OpenFile(request()), &paths, None, &provider)
                .expect("dialog should dispatch");

        assert_eq!(
            result,
            DialogResult::Selected {
                paths: vec![cefari_core::DialogSelectedPath {
                    path: selected.to_string_lossy().into_owned(),
                    name: "report.txt".to_owned(),
                    kind: FileKind::File,
                }],
            }
        );
    }

    #[test]
    fn validates_dialog_filters() {
        let paths = fixture_paths("validates_dialog_filters");
        for filter in [
            DialogFilter {
                name: String::new(),
                extensions: vec!["txt".to_owned()],
            },
            DialogFilter {
                name: "Text".to_owned(),
                extensions: Vec::new(),
            },
            DialogFilter {
                name: "Text".to_owned(),
                extensions: vec![".txt".to_owned()],
            },
            DialogFilter {
                name: "Text".to_owned(),
                extensions: vec!["*.txt".to_owned()],
            },
            DialogFilter {
                name: "Text".to_owned(),
                extensions: vec![format!("nested{MAIN_SEPARATOR_STR}txt")],
            },
        ] {
            let mut request = request();
            request.filters = vec![filter];

            assert!(
                invocation_for_command(&DialogCommand::OpenFile(request), &paths).is_err(),
                "invalid filter should be rejected"
            );
        }
    }

    #[test]
    fn validates_title_and_default_name() {
        let paths = fixture_paths("validates_title_and_default_name");
        let mut blank_title = request();
        blank_title.title = Some("  ".to_owned());
        assert!(invocation_for_command(&DialogCommand::OpenFile(blank_title), &paths).is_err());

        let mut nul_title = request();
        nul_title.title = Some("Bad\0Title".to_owned());
        assert!(invocation_for_command(&DialogCommand::OpenFile(nul_title), &paths).is_err());

        let mut blank_name = request();
        blank_name.default_name = Some(String::new());
        assert!(invocation_for_command(&DialogCommand::OpenFile(blank_name), &paths).is_err());

        let mut path_name = request();
        path_name.default_name = Some("nested/report.txt".to_owned());
        assert!(invocation_for_command(&DialogCommand::OpenFile(path_name), &paths).is_err());
    }

    #[test]
    fn marks_missing_save_path_as_file_selection() {
        let paths = fixture_paths("marks_missing_save_path_as_file_selection");
        let selected = paths.data_dir.join("new-report.txt");
        let provider = FakeDialogProvider {
            response: Some(vec![selected.clone()]),
            invocations: RefCell::new(Vec::new()),
        };

        let result =
            dispatch_with_provider(&DialogCommand::SaveFile(request()), &paths, None, &provider)
                .expect("dialog should dispatch");

        assert_eq!(
            result,
            DialogResult::Selected {
                paths: vec![cefari_core::DialogSelectedPath {
                    path: selected.to_string_lossy().into_owned(),
                    name: "new-report.txt".to_owned(),
                    kind: FileKind::File,
                }],
            }
        );
    }

    #[test]
    fn resolves_app_data_default_directories() {
        let paths = fixture_paths("resolves_app_data_default_directories");
        let invocation = invocation_for_command(&DialogCommand::OpenFile(request()), &paths)
            .expect("dialog command should normalize");

        assert_eq!(
            invocation.default_directory,
            Some(paths.data_dir.join("exports"))
        );
    }

    #[test]
    fn rejects_app_data_default_directory_traversal() {
        let paths = fixture_paths("rejects_app_data_default_directory_traversal");
        let mut request = request();
        request.default_directory = Some(DialogDefaultDirectory::AppData {
            path: Some("../secret".to_owned()),
        });

        let error = invocation_for_command(&DialogCommand::OpenFile(request), &paths)
            .expect_err("traversal should fail");

        assert!(error.to_string().contains("parent path traversal"));
    }

    #[test]
    fn passes_native_default_directories_through() {
        let paths = fixture_paths("passes_native_default_directories_through");
        let mut request = request();
        request.default_directory = Some(DialogDefaultDirectory::Native {
            path: "/tmp/native-default".to_owned(),
        });

        let invocation = invocation_for_command(&DialogCommand::OpenFile(request), &paths)
            .expect("dialog command should normalize");

        assert_eq!(
            invocation.default_directory,
            Some(PathBuf::from("/tmp/native-default"))
        );
    }

    fn request() -> DialogRequest {
        DialogRequest {
            title: Some("Choose File".to_owned()),
            filters: vec![DialogFilter {
                name: "Text".to_owned(),
                extensions: vec!["txt".to_owned()],
            }],
            default_directory: Some(DialogDefaultDirectory::AppData {
                path: Some("exports".to_owned()),
            }),
            default_name: Some("report.txt".to_owned()),
            modality: Some(DialogModality::Window),
            can_create_directories: Some(true),
        }
    }

    fn fixture_paths(name: &str) -> RuntimePaths {
        let root = std::env::temp_dir().join(format!(
            "cefari-desktop-dialogs-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        RuntimePaths {
            config_dir: root.join("config"),
            config_file: root.join("config").join("cefari.json"),
            data_dir: root.join("data"),
            cache_dir: root.join("cache"),
            log_dir: root.join("data").join("logs"),
            resource_dir: root.join("data").join("resources"),
            update_dir: root.join("data").join("updates"),
        }
    }
}
