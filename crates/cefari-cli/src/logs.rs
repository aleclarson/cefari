use std::{
    fs::{self, File},
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    thread,
    time::Duration,
};

use anyhow::{Context, Result};
use cefari_core::{AppIdentity, LogFileConfig, RuntimeLogConfig, RuntimePaths};
use clap::ValueEnum;

#[derive(Debug, Clone, Copy, Eq, PartialEq, ValueEnum)]
pub enum LogKind {
    All,
    App,
    Daemon,
    Rust,
}

pub fn print_logs(kind: LogKind, tail: usize, follow: bool, path_only: bool) -> Result<()> {
    let paths = RuntimePaths::resolve(&AppIdentity::cefari())?;
    let config = RuntimeLogConfig::new(&paths);

    if path_only {
        println!("{}", config.directory.display());
        return Ok(());
    }

    let streams = selected_streams(&config, kind);
    print_existing_logs(&streams, tail)?;

    if follow {
        follow_logs(&streams)?;
    }

    Ok(())
}

fn selected_streams(config: &RuntimeLogConfig, kind: LogKind) -> Vec<&LogFileConfig> {
    match kind {
        LogKind::All => config.streams().into_iter().collect(),
        LogKind::App => vec![&config.app],
        LogKind::Daemon => vec![&config.daemon],
        LogKind::Rust => vec![&config.rust],
    }
}

fn print_existing_logs(streams: &[&LogFileConfig], tail: usize) -> Result<()> {
    for (index, stream) in streams.iter().enumerate() {
        if index > 0 {
            println!();
        }
        println!("== {} ==", stream.file_name);

        let files = stream_files(stream)?;
        if files.is_empty() {
            println!("no log files found at {}", stream.directory.display());
            continue;
        }

        let lines = tail_lines(&files, tail)?;
        if lines.is_empty() {
            println!("log files are empty");
            continue;
        }
        for line in lines {
            println!("{line}");
        }
    }

    Ok(())
}

fn stream_files(stream: &LogFileConfig) -> Result<Vec<PathBuf>> {
    let prefix = stream.rotated_file_prefix();
    let mut files = Vec::new();

    match fs::read_dir(&stream.directory) {
        Ok(entries) => {
            for entry in entries {
                let entry = entry.with_context(|| {
                    format!(
                        "failed to read log directory {}",
                        stream.directory.display()
                    )
                })?;
                let file_name = entry.file_name();
                let Some(file_name) = file_name.to_str() else {
                    continue;
                };
                if file_name == stream.file_name || file_name.starts_with(&prefix) {
                    files.push(entry.path());
                }
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(error).with_context(|| {
                format!(
                    "failed to read log directory {}",
                    stream.directory.display()
                )
            });
        }
    }

    files.sort_by(|left, right| {
        let left_current = left
            .file_name()
            .is_some_and(|name| name == stream.file_name.as_str());
        let right_current = right
            .file_name()
            .is_some_and(|name| name == stream.file_name.as_str());
        match (left_current, right_current) {
            (true, false) => std::cmp::Ordering::Greater,
            (false, true) => std::cmp::Ordering::Less,
            _ => match (
                rotated_numeric_suffix(left, stream),
                rotated_numeric_suffix(right, stream),
            ) {
                (Some(left), Some(right)) => right.cmp(&left),
                _ => left.cmp(right),
            },
        }
    });
    Ok(files)
}

fn rotated_numeric_suffix(path: &Path, stream: &LogFileConfig) -> Option<usize> {
    path.file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_prefix(&stream.rotated_file_prefix()))
        .and_then(|suffix| suffix.parse().ok())
}

fn tail_lines(files: &[PathBuf], tail: usize) -> Result<Vec<String>> {
    let mut lines = Vec::new();

    for path in files {
        let contents = fs::read_to_string(path)
            .with_context(|| format!("failed to read log file {}", path.display()))?;
        lines.extend(contents.lines().map(ToOwned::to_owned));
    }

    if tail == 0 || lines.len() <= tail {
        Ok(lines)
    } else {
        Ok(lines.split_off(lines.len() - tail))
    }
}

fn follow_logs(streams: &[&LogFileConfig]) -> Result<()> {
    let mut offsets = streams
        .iter()
        .map(|stream| {
            let path = latest_stream_file(stream).unwrap_or_else(|| stream.file_path());
            let offset = file_len(&path).unwrap_or(0);
            (stream.file_name.clone(), path, offset)
        })
        .collect::<Vec<_>>();

    loop {
        for (name, path, offset) in &mut offsets {
            if let Some(chunk) = read_appended(path, *offset)? {
                *offset += u64::try_from(chunk.len()).unwrap_or(u64::MAX);
                if !chunk.is_empty() {
                    print!("{chunk}");
                    if !chunk.ends_with('\n') {
                        println!();
                    }
                }
            } else if path.exists() {
                *offset = 0;
                eprintln!("following recreated log file {name}");
            }
        }
        thread::sleep(Duration::from_secs(1));
    }
}

fn latest_stream_file(stream: &LogFileConfig) -> Option<PathBuf> {
    stream_files(stream)
        .ok()
        .and_then(|files| files.into_iter().last())
}

fn file_len(path: &Path) -> Result<u64> {
    Ok(fs::metadata(path)
        .with_context(|| format!("failed to stat log file {}", path.display()))?
        .len())
}

fn read_appended(path: &Path, offset: u64) -> Result<Option<String>> {
    let Ok(mut file) = File::open(path) else {
        return Ok(None);
    };
    let len = file
        .metadata()
        .with_context(|| format!("failed to stat log file {}", path.display()))?
        .len();
    if len < offset {
        return Ok(None);
    }
    if len == offset {
        return Ok(Some(String::new()));
    }

    file.seek(SeekFrom::Start(offset))
        .with_context(|| format!("failed to seek log file {}", path.display()))?;
    let mut chunk = String::new();
    file.read_to_string(&mut chunk)
        .with_context(|| format!("failed to read log file {}", path.display()))?;
    Ok(Some(chunk))
}

#[cfg(test)]
mod tests {
    use super::{stream_files, tail_lines};
    use cefari_core::{AppIdentity, RuntimeLogConfig, RuntimePaths};
    use std::fs;

    #[test]
    fn finds_current_and_rotated_stream_files() {
        let root =
            std::env::temp_dir().join(format!("cefari-cli-log-files-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();

        let mut stream =
            RuntimeLogConfig::new(&RuntimePaths::resolve(&AppIdentity::cefari()).unwrap()).app;
        stream.directory.clone_from(&root);

        fs::write(root.join("app.log.2026-06-01"), "old").unwrap();
        fs::write(root.join("app.log"), "current").unwrap();
        fs::write(root.join("daemon.log"), "other").unwrap();

        let files = stream_files(&stream).unwrap();

        assert_eq!(files.len(), 2);
        assert!(files[0].ends_with("app.log.2026-06-01"));
        assert!(files[1].ends_with("app.log"));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn tails_across_multiple_files() {
        let root = std::env::temp_dir().join(format!("cefari-cli-log-tail-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let first = root.join("app.log.2026-06-01");
        let second = root.join("app.log");
        fs::write(&first, "one\ntwo\n").unwrap();
        fs::write(&second, "three\nfour\n").unwrap();

        let lines = tail_lines(&[first, second], 3).unwrap();

        assert_eq!(lines, ["two", "three", "four"]);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn orders_numbered_rotations_from_oldest_to_current() {
        let root = std::env::temp_dir().join(format!(
            "cefari-cli-log-rotation-order-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();

        let mut stream =
            RuntimeLogConfig::new(&RuntimePaths::resolve(&AppIdentity::cefari()).unwrap()).daemon;
        stream.directory.clone_from(&root);

        fs::write(root.join("daemon.log.2"), "old").unwrap();
        fs::write(root.join("daemon.log.1"), "newer").unwrap();
        fs::write(root.join("daemon.log"), "current").unwrap();

        let files = stream_files(&stream).unwrap();

        assert!(files[0].ends_with("daemon.log.2"));
        assert!(files[1].ends_with("daemon.log.1"));
        assert!(files[2].ends_with("daemon.log"));

        fs::remove_dir_all(root).unwrap();
    }
}
