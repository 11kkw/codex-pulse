use std::{
    fs::{create_dir_all, OpenOptions},
    io::Write,
    path::PathBuf,
};

use chrono::Local;

fn log_directory() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .map(|path| path.join("Codex Pulse").join("logs"))
}

pub fn log_path() -> Option<PathBuf> {
    log_directory().map(|path| path.join("codex-pulse.log"))
}

pub fn write(message: impl AsRef<str>) {
    let Some(directory) = log_directory() else {
        return;
    };
    if create_dir_all(&directory).is_err() {
        return;
    }
    let path = directory.join("codex-pulse.log");
    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) else {
        return;
    };
    let _ = writeln!(
        file,
        "{} {}",
        Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
        message.as_ref()
    );
}
