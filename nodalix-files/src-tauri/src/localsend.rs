use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::net::{IpAddr, Ipv4Addr};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Clone)]
pub struct LocalSendInfo {
    pub available: bool,
    pub command: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LocalSendDevice {
    pub id: String,
    pub alias: String,
    pub device_model: Option<String>,
    pub device_type: Option<String>,
    pub fingerprint: Option<String>,
    pub ip: String,
    pub port: u16,
    pub protocol: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LocalSendInfoResponse {
    alias: String,
    device_model: Option<String>,
    device_type: Option<String>,
    fingerprint: Option<String>,
}

fn command_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn flatpak_localsend() -> bool {
    Command::new("flatpak")
        .args(["info", "org.localsend.localsend"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn desktop_file_exists() -> bool {
    let dirs = [
        "/usr/share/applications",
        "/var/lib/flatpak/exports/share/applications",
        "/.local/share/applications",
    ];
    let home = std::env::var("HOME").unwrap_or_default();
    for dir in dirs {
        let path = if dir.starts_with('/') {
            Path::new(dir).join("localsend.desktop")
        } else {
            Path::new(&home).join(".local/share/applications/localsend.desktop")
        };
        if path.is_file() {
            return true;
        }
    }
    for name in ["localsend.desktop", "org.localsend.localsend.desktop"] {
        if Path::new("/usr/share/applications").join(name).is_file() {
            return true;
        }
    }
    false
}

pub fn detect_localsend() -> LocalSendInfo {
    if command_exists("localsend") {
        return LocalSendInfo {
            available: true,
            command: Some("localsend".into()),
        };
    }
    if flatpak_localsend() {
        return LocalSendInfo {
            available: true,
            command: Some("flatpak run org.localsend.localsend".into()),
        };
    }
    if desktop_file_exists() {
        return LocalSendInfo {
            available: true,
            command: Some("xdg-open".into()),
        };
    }
    LocalSendInfo {
        available: false,
        command: None,
    }
}

fn local_ipv4_candidates() -> Vec<Ipv4Addr> {
    let output = Command::new("ip").args(["neigh", "show"]).output();
    let Ok(output) = output else {
        return Vec::new();
    };
    let text = String::from_utf8_lossy(&output.stdout);
    let mut addresses = Vec::new();
    for line in text.lines() {
        let Some(addr) = line.split_whitespace().next() else {
            continue;
        };
        let Ok(IpAddr::V4(ipv4)) = addr.parse::<IpAddr>() else {
            continue;
        };
        if !addresses.contains(&ipv4) {
            addresses.push(ipv4);
        }
    }
    addresses
}

fn curl_json(url: &str, timeout: &str) -> Option<Value> {
    let output = Command::new("curl")
        .args(["-skS", "--max-time", timeout, url])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    serde_json::from_slice(&output.stdout).ok()
}

fn probe_device(ip: Ipv4Addr, protocol: &str) -> Option<LocalSendDevice> {
    let url = format!("{protocol}://{ip}:53317/api/localsend/v2/info");
    let value = curl_json(&url, "0.25")?;
    let info: LocalSendInfoResponse = serde_json::from_value(value).ok()?;
    let id = info
        .fingerprint
        .clone()
        .unwrap_or_else(|| format!("{protocol}-{ip}-53317"));
    Some(LocalSendDevice {
        id,
        alias: info.alias,
        device_model: info.device_model,
        device_type: info.device_type,
        fingerprint: info.fingerprint,
        ip: ip.to_string(),
        port: 53317,
        protocol: protocol.into(),
    })
}

fn discover_localsend_devices_blocking() -> Result<Vec<LocalSendDevice>, String> {
    crate::platform::debug_log("localsend discover start");
    if !command_exists("curl") {
        return Err("curl is required for LocalSend device discovery".into());
    }

    let mut devices = BTreeMap::<String, LocalSendDevice>::new();
    for ip in local_ipv4_candidates() {
        for protocol in ["https", "http"] {
            if let Some(device) = probe_device(ip, protocol) {
                devices.entry(device.id.clone()).or_insert(device);
                break;
            }
        }
    }

    let devices: Vec<_> = devices.into_values().collect();
    crate::platform::debug_log(&format!("localsend discover found {}", devices.len()));
    Ok(devices)
}

#[tauri::command]
pub async fn discover_localsend_devices() -> Result<Vec<LocalSendDevice>, String> {
    tauri::async_runtime::spawn_blocking(discover_localsend_devices_blocking)
        .await
        .map_err(|e| e.to_string())?
}

#[derive(Clone)]
struct UploadFile {
    id: String,
    path: PathBuf,
    name: String,
    size: u64,
    modified: Option<String>,
}

fn collect_upload_files(
    path: &Path,
    root_name: Option<&str>,
    out: &mut Vec<UploadFile>,
) -> Result<(), String> {
    let metadata =
        fs::symlink_metadata(path).map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
    if metadata.is_dir() {
        let base = root_name
            .map(ToOwned::to_owned)
            .or_else(|| {
                path.file_name()
                    .map(|name| name.to_string_lossy().to_string())
            })
            .unwrap_or_else(|| "Folder".into());
        for entry in
            fs::read_dir(path).map_err(|e| format!("Cannot read folder {}: {e}", path.display()))?
        {
            let entry = entry.map_err(|e| e.to_string())?;
            let child = entry.path();
            let child_name = entry.file_name().to_string_lossy().to_string();
            collect_upload_files(&child, Some(&format!("{base}/{child_name}")), out)?;
        }
        return Ok(());
    }
    if !metadata.is_file() {
        return Ok(());
    }
    let name = root_name
        .map(ToOwned::to_owned)
        .or_else(|| {
            path.file_name()
                .map(|name| name.to_string_lossy().to_string())
        })
        .ok_or_else(|| "Invalid file path".to_string())?;
    let id = format!("file-{}", out.len());
    let modified = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| format!("{}Z", humantime_utc(duration.as_secs())));
    out.push(UploadFile {
        id,
        path: path.to_path_buf(),
        name,
        size: metadata.len(),
        modified,
    });
    Ok(())
}

fn humantime_utc(secs: u64) -> String {
    let output = Command::new("date")
        .args(["-u", "-d", &format!("@{secs}"), "+%Y-%m-%dT%H:%M:%S"])
        .output();
    output
        .ok()
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "1970-01-01T00:00:00".into())
}

fn percent_encode(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

fn post_json(url: &str, body: &Value) -> Result<Value, String> {
    crate::platform::debug_log(&format!("localsend post {url}"));
    let mut child = Command::new("curl")
        .args([
            "-skS",
            "--fail-with-body",
            "-X",
            "POST",
            "-H",
            "Content-Type: application/json",
            "--data-binary",
            "@-",
            url,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start curl: {e}"))?;
    if let Some(stdin) = child.stdin.as_mut() {
        stdin
            .write_all(body.to_string().as_bytes())
            .map_err(|e| e.to_string())?;
    }
    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        return Err(if stdout.is_empty() { stderr } else { stdout });
    }
    serde_json::from_slice(&output.stdout).map_err(|e| format!("Invalid LocalSend response: {e}"))
}

fn upload_file(url: &str, path: &Path) -> Result<(), String> {
    crate::platform::debug_log(&format!("localsend upload {}", path.display()));
    let output = Command::new("curl")
        .args([
            "-skS",
            "--fail-with-body",
            "-X",
            "POST",
            "--data-binary",
            &format!("@{}", path.display()),
            url,
        ])
        .output()
        .map_err(|e| format!("Failed to start curl: {e}"))?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Err(if stdout.is_empty() { stderr } else { stdout })
}

fn send_to_localsend_device_blocking(
    paths: Vec<String>,
    device: LocalSendDevice,
) -> Result<(), String> {
    crate::platform::debug_log(&format!(
        "localsend send start {} paths to {} {}:{}",
        paths.len(),
        device.alias,
        device.ip,
        device.port
    ));
    if paths.is_empty() {
        return Err("No files selected".into());
    }
    if !command_exists("curl") {
        return Err("curl is required to send files with LocalSend".into());
    }

    let mut files = Vec::new();
    for path in &paths {
        collect_upload_files(Path::new(path), None, &mut files)?;
    }
    if files.is_empty() {
        return Err("No regular files to send".into());
    }

    let mut file_map = serde_json::Map::new();
    for file in &files {
        let mut metadata = serde_json::Map::new();
        if let Some(modified) = &file.modified {
            metadata.insert("modified".into(), json!(modified));
        }
        file_map.insert(
            file.id.clone(),
            json!({
                "id": file.id,
                "fileName": file.name,
                "size": file.size,
                "fileType": "application/octet-stream",
                "sha256": null,
                "preview": null,
                "metadata": metadata,
            }),
        );
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    let body = json!({
        "info": {
            "alias": "Nodalix Files",
            "version": "2.0",
            "deviceModel": "Nodalix OS",
            "deviceType": "desktop",
            "fingerprint": format!("nodalix-files-{now}"),
            "port": 53317,
            "protocol": "https",
            "download": false,
        },
        "files": file_map,
    });

    let base = format!("{}://{}:{}", device.protocol, device.ip, device.port);
    let prepare_url = format!("{base}/api/localsend/v2/prepare-upload");
    let response = post_json(&prepare_url, &body)?;
    let session_id = response
        .get("sessionId")
        .and_then(Value::as_str)
        .ok_or_else(|| "LocalSend did not return a session id".to_string())?;
    let tokens = response
        .get("files")
        .and_then(Value::as_object)
        .ok_or_else(|| "LocalSend did not accept any files".to_string())?;

    for file in &files {
        let token = tokens
            .get(&file.id)
            .and_then(Value::as_str)
            .ok_or_else(|| format!("LocalSend did not accept {}", file.name))?;
        let upload_url = format!(
            "{base}/api/localsend/v2/upload?sessionId={}&fileId={}&token={}",
            percent_encode(session_id),
            percent_encode(&file.id),
            percent_encode(token),
        );
        upload_file(&upload_url, &file.path)?;
    }

    crate::platform::debug_log("localsend send done");
    Ok(())
}

#[tauri::command]
pub async fn send_to_localsend_device(
    paths: Vec<String>,
    device: LocalSendDevice,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || send_to_localsend_device_blocking(paths, device))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn get_localsend_info() -> LocalSendInfo {
    detect_localsend()
}

#[tauri::command]
pub fn send_with_localsend(paths: Vec<String>) -> Result<(), String> {
    let info = detect_localsend();
    if !info.available {
        return Err("LocalSend is not installed".into());
    }
    let cmd = info.command.unwrap_or_else(|| "localsend".into());
    if cmd.starts_with("flatpak") {
        let mut args: Vec<&str> = vec!["run", "org.localsend.localsend"];
        for p in &paths {
            args.push(p);
        }
        Command::new("flatpak")
            .args(args)
            .spawn()
            .map_err(|e| e.to_string())?;
    } else if cmd == "xdg-open" {
        for p in paths {
            Command::new("xdg-open")
                .arg(p)
                .spawn()
                .map_err(|e| e.to_string())?;
        }
    } else {
        let mut c = Command::new(&cmd);
        c.args(paths);
        c.spawn().map_err(|e| e.to_string())?;
    }
    Ok(())
}
