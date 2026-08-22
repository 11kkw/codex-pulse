use std::{
    env, fs,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    time::SystemTime,
};

use serde_json::{json, Value};

pub struct AppServerClient {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
}

impl AppServerClient {
    pub fn connect() -> Result<Self, String> {
        let executable = find_codex_executable()?;
        crate::logging::write(format!(
            "starting Codex app-server: {}",
            executable.display()
        ));
        let mut command = Command::new(&executable);
        command
            .arg("app-server")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        hide_console(&mut command);
        let mut child = command.spawn().map_err(|error| {
            format!(
                "Codex app-server를 시작할 수 없습니다 ({}): {error}",
                executable.display()
            )
        })?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "Codex app-server stdin을 열 수 없습니다.".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "Codex app-server stdout을 열 수 없습니다.".to_string())?;

        let mut client = Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
            next_id: 1,
        };

        client.request_with_params(
            "initialize",
            json!({
                "clientInfo": {
                    "name": "codex_usage_widget",
                    "title": "Codex Pulse",
                    "version": env!("CARGO_PKG_VERSION")
                },
                "capabilities": {
                    "experimentalApi": true
                }
            }),
        )?;
        client.send(json!({ "method": "initialized", "params": {} }))?;
        Ok(client)
    }

    pub fn request(&mut self, method: &str) -> Result<Value, String> {
        self.request_with_params(method, json!({}))
    }

    pub(crate) fn request_with_params(
        &mut self,
        method: &str,
        params: Value,
    ) -> Result<Value, String> {
        let id = self.next_id;
        self.next_id += 1;
        self.send(json!({ "method": method, "id": id, "params": params }))?;

        loop {
            let mut line = String::new();
            let bytes = self
                .stdout
                .read_line(&mut line)
                .map_err(|error| format!("Codex app-server 응답을 읽지 못했습니다: {error}"))?;
            if bytes == 0 {
                return Err("Codex app-server 연결이 종료되었습니다.".into());
            }

            let message: Value = serde_json::from_str(&line).map_err(|error| {
                format!("Codex app-server 응답 형식이 올바르지 않습니다: {error}")
            })?;
            if message.get("id").and_then(Value::as_u64) != Some(id) {
                continue;
            }

            if let Some(error) = message.get("error") {
                return Err(error
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("Codex app-server 요청이 실패했습니다.")
                    .to_string());
            }

            return message
                .get("result")
                .cloned()
                .ok_or_else(|| "Codex app-server 응답에 result가 없습니다.".to_string());
        }
    }

    fn send(&mut self, message: Value) -> Result<(), String> {
        writeln!(self.stdin, "{message}")
            .and_then(|_| self.stdin.flush())
            .map_err(|error| format!("Codex app-server에 요청을 보내지 못했습니다: {error}"))
    }
}

fn find_codex_executable() -> Result<PathBuf, String> {
    if let Some(path) = env::var_os("CODEX_EXECUTABLE").map(PathBuf::from) {
        if path.is_file() {
            return Ok(path);
        }
    }

    #[cfg(windows)]
    if let Some(local_app_data) = env::var_os("LOCALAPPDATA").map(PathBuf::from) {
        let mut desktop_candidates = Vec::new();
        collect_versioned_executables(
            &local_app_data.join("OpenAI").join("Codex").join("bin"),
            Path::new("codex.exe"),
            &mut desktop_candidates,
        );
        if let Some(path) = newest_candidate(desktop_candidates) {
            return Ok(path);
        }
    }

    #[cfg(windows)]
    if let Some(user_profile) = env::var_os("USERPROFILE").map(PathBuf::from) {
        let mut extension_candidates = Vec::new();
        for extensions_dir in [
            user_profile.join(".windsurf").join("extensions"),
            user_profile.join(".vscode").join("extensions"),
        ] {
            collect_matching_extension_executables(&extensions_dir, &mut extension_candidates);
        }
        if let Some(path) = newest_candidate(extension_candidates) {
            return Ok(path);
        }
    }

    #[cfg(target_os = "macos")]
    if let Some(home) = env::var_os("HOME").map(PathBuf::from) {
        let mut extension_candidates = Vec::new();
        for extensions_dir in [
            home.join(".vscode").join("extensions"),
            home.join(".vscode-insiders").join("extensions"),
            home.join(".windsurf").join("extensions"),
        ] {
            collect_matching_extension_executables(&extensions_dir, &mut extension_candidates);
        }
        if let Some(path) = newest_candidate(extension_candidates) {
            return Ok(path);
        }
    }

    #[cfg(target_os = "macos")]
    for path in ["/opt/homebrew/bin/codex", "/usr/local/bin/codex"] {
        let candidate = PathBuf::from(path);
        if candidate.is_file() {
            return Ok(candidate);
        }
    }

    #[cfg(not(windows))]
    {
        let mut which_command = Command::new("which");
        which_command.arg("codex");
        if let Ok(output) = which_command.output() {
            if output.status.success() {
                if let Some(path) = String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .map(str::trim)
                    .find(|line| !line.is_empty())
                    .map(PathBuf::from)
                    .filter(|path| path.is_file())
                {
                    return Ok(path);
                }
            }
        }
    }

    #[cfg(windows)]
    {
        let mut where_command = Command::new("where.exe");
        where_command.arg("codex.exe");
        hide_console(&mut where_command);
        if let Ok(output) = where_command.output() {
            if output.status.success() {
                if let Some(path) = String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .map(PathBuf::from)
                    .find(|path| {
                        path.is_file()
                            && !path
                                .to_string_lossy()
                                .to_ascii_lowercase()
                                .contains("\\windowsapps\\")
                    })
                {
                    return Ok(path);
                }
            }
        }
    }

    Err(format!(
        "Codex 실행 파일을 찾지 못했습니다. Codex CLI를 설치하거나 CODEX_EXECUTABLE 환경 변수에 {} 경로를 지정해 주세요.",
        if cfg!(windows) { "codex.exe" } else { "codex" }
    ))
}

#[cfg(windows)]
fn hide_console(command: &mut Command) {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn hide_console(_command: &mut Command) {}

fn newest_candidate(candidates: Vec<PathBuf>) -> Option<PathBuf> {
    candidates.into_iter().max_by_key(|path| {
        fs::metadata(path)
            .and_then(|metadata| metadata.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH)
    })
}

fn collect_versioned_executables(
    parent: &Path,
    executable_suffix: &Path,
    candidates: &mut Vec<PathBuf>,
) {
    let Ok(entries) = fs::read_dir(parent) else {
        return;
    };

    for entry in entries.flatten() {
        let candidate = entry.path().join(executable_suffix);
        if candidate.is_file() {
            candidates.push(candidate);
        }
    }
}

fn collect_matching_extension_executables(parent: &Path, candidates: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(parent) else {
        return;
    };

    for entry in entries.flatten() {
        let file_name = entry.file_name();
        if !file_name.to_string_lossy().starts_with("openai.chatgpt-") {
            continue;
        }

        let extension = entry.path();
        #[cfg(windows)]
        let relative_candidates = ["bin/windows-x86_64/codex.exe"];
        #[cfg(target_os = "macos")]
        let relative_candidates = ["bin/macos-aarch64/codex", "bin/macos-x86_64/codex"];
        #[cfg(all(not(windows), not(target_os = "macos")))]
        let relative_candidates = ["bin/linux-x86_64/codex", "bin/linux-aarch64/codex"];

        for relative in relative_candidates {
            let candidate = extension.join(relative);
            if candidate.is_file() {
                candidates.push(candidate);
            }
        }
    }
}

impl Drop for AppServerClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
