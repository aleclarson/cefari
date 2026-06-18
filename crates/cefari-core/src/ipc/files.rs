use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "file", content = "payload", rename_all = "camelCase")]
pub enum FilesCommand {
    AppDataDir,
    ReadFile(FileReadRequest),
    WriteFile(FileWriteRequest),
    Readdir(ReadDirRequest),
    Mkdir(MkdirRequest),
    Rm(RmRequest),
    Rename(RenameRequest),
    CopyFile(CopyFileRequest),
    Stat(FilePathRequest),
    Access(FilePathRequest),
    Exists(FilePathRequest),
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct FilePathRequest {
    pub path: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct FileReadRequest {
    pub path: String,
    pub encoding: Option<FileEncoding>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct FileWriteRequest {
    pub path: String,
    pub contents: FileContents,
    pub options: FileWriteOptions,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum FileContents {
    Text(String),
    Base64(String),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum FileEncoding {
    Utf8,
    Base64,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct FileWriteOptions {
    pub create_parents: bool,
    pub overwrite: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ReadDirRequest {
    pub path: String,
    pub with_file_types: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct MkdirRequest {
    pub path: String,
    pub recursive: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RmRequest {
    pub path: String,
    pub recursive: bool,
    pub force: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RenameRequest {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CopyFileRequest {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "result", content = "payload", rename_all = "camelCase")]
pub enum FileResult {
    AppDataDir(AppDataDirInfo),
    Text { contents: String },
    Base64 { contents: String },
    DirEntries { entries: Vec<DirEntry> },
    Stat(FileStat),
    Access { ok: bool },
    Exists { exists: bool },
    Written(FileWriteResult),
    Empty,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AppDataDirInfo {
    pub root_kind: String,
    pub display_path: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DirEntry {
    pub name: String,
    pub path: String,
    pub kind: FileKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct FileStat {
    pub path: String,
    pub kind: FileKind,
    pub size: f64,
    pub modified_at_ms: Option<f64>,
    pub created_at_ms: Option<f64>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum FileKind {
    File,
    Directory,
    Symlink,
    Other,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct FileWriteResult {
    pub path: String,
    pub bytes_written: f64,
}
