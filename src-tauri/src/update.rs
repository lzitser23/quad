//! In-app self-update, modeled on milim's updater (github.com/oshtz/milim).
//!
//! The frontend finds the newer GitHub release and hands the platform asset's
//! URLs to [`download_update`], which streams it down with progress, verifies
//! its SHA-256 sidecar, and stages it under the app's local data directory.
//! [`apply_update`] then spawns a detached script that waits for the app to
//! exit, swaps the executable (Windows) or app bundle (macOS) with a backup,
//! and relaunches. A failed swap restores the backup and leaves an error
//! marker that [`take_update_recovery_error`] surfaces next launch.

use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::ipc::Channel;
use tauri::Manager;

const UPDATE_DIR_NAME: &str = "updates";
const RECOVERY_ERROR_NAME: &str = "install-error.txt";
const MAX_PACKAGE_BYTES: usize = 512 * 1024 * 1024;
const MAX_CHECKSUM_BYTES: usize = 1024 * 1024;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProgress {
    phase: &'static str,
    downloaded_bytes: u64,
    total_bytes: Option<u64>,
}

fn send_progress(
    channel: &Channel<UpdateProgress>,
    phase: &'static str,
    downloaded_bytes: u64,
    total_bytes: Option<u64>,
) {
    let _ = channel.send(UpdateProgress {
        phase,
        downloaded_bytes,
        total_bytes,
    });
}

fn update_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let data_dir = app
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("could not resolve app data directory: {e}"))?;
    Ok(data_dir.join(UPDATE_DIR_NAME))
}

/// Which update asset the frontend should look for.
#[tauri::command]
pub fn update_platform() -> &'static str {
    if cfg!(target_os = "macos") {
        "darwin"
    } else if cfg!(target_os = "windows") {
        "win32"
    } else {
        "linux"
    }
}

/// The error marker a failed swap script leaves behind, consumed on read so
/// the next launch reports it exactly once.
#[tauri::command]
pub fn take_update_recovery_error(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let marker = update_dir(&app)?.join(RECOVERY_ERROR_NAME);
    match fs::read_to_string(&marker) {
        Ok(contents) => {
            let _ = fs::remove_file(marker);
            let message = contents.trim();
            Ok((!message.is_empty()).then(|| message.to_string()))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

/// Asset names are release-asset basenames; anything path-like is hostile.
fn validate_asset_name(name: &str) -> Result<(), String> {
    if name.trim().is_empty()
        || name.contains('/')
        || name.contains('\\')
        || name.starts_with('.')
    {
        return Err("invalid update file name".into());
    }
    let lower = name.to_ascii_lowercase();
    if !(lower.ends_with(".exe") || lower.ends_with(".app.zip")) {
        return Err("unsupported update file type".into());
    }
    Ok(())
}

/// Only GitHub release URLs over https — the URLs come from the webview.
fn validate_download_url(url: &str, label: &str) -> Result<String, String> {
    let parsed = reqwest::Url::parse(url).map_err(|e| format!("{label} is invalid: {e}"))?;
    if parsed.scheme() != "https" {
        return Err(format!("{label} must use https"));
    }
    match parsed.host_str() {
        Some("github.com" | "api.github.com") => Ok(parsed.to_string()),
        _ => Err(format!("{label} must be a GitHub release URL")),
    }
}

/// First 64-hex-digit token in `line`, i.e. a SHA-256 in any common checksum
/// layout ("<hash>  <name>", "<name>: <hash>", or a bare hash).
fn first_sha256_hex(line: &str) -> Option<String> {
    line.split(|ch: char| !ch.is_ascii_hexdigit())
        .find(|part| part.len() == 64)
        .map(|part| part.to_ascii_lowercase())
}

fn expected_sha256(checksum_text: &str, asset_name: &str) -> Option<String> {
    let lines: Vec<&str> = checksum_text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    lines
        .iter()
        .copied()
        .find(|l| l.contains(asset_name) && first_sha256_hex(l).is_some())
        .or_else(|| (lines.len() == 1).then(|| lines[0]))
        .and_then(first_sha256_hex)
}

fn verify_checksum(bytes: &[u8], checksum_text: &str, asset_name: &str) -> Result<(), String> {
    let expected = expected_sha256(checksum_text, asset_name)
        .ok_or_else(|| format!("checksum file has no SHA-256 for {asset_name}"))?;
    let actual = format!("{:x}", Sha256::digest(bytes));
    if actual != expected {
        return Err(format!("checksum mismatch for {asset_name}"));
    }
    Ok(())
}

async fn fetch_bytes(
    client: &reqwest::Client,
    url: &str,
    label: &str,
    max_bytes: usize,
    on_progress: Option<&Channel<UpdateProgress>>,
) -> Result<Vec<u8>, String> {
    let mut response = client
        .get(url)
        .header(reqwest::header::ACCEPT, "application/octet-stream")
        .send()
        .await
        .map_err(|e| format!("{label} download failed: {e}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("{label} download failed ({status})"));
    }
    let total = response.content_length();
    if total.is_some_and(|t| t > max_bytes as u64) {
        return Err(format!("{label} is too large"));
    }
    let mut bytes = Vec::new();
    let mut last_reported = 0u64;
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| format!("{label} download failed: {e}"))?
    {
        if bytes.len().saturating_add(chunk.len()) > max_bytes {
            return Err(format!("{label} is too large"));
        }
        bytes.extend_from_slice(&chunk);
        let downloaded = bytes.len() as u64;
        // Report at most every 256 KiB so the channel doesn't flood the webview.
        if let Some(channel) = on_progress {
            if downloaded.saturating_sub(last_reported) >= 256 * 1024 {
                send_progress(channel, "downloading", downloaded, total);
                last_reported = downloaded;
            }
        }
    }
    if bytes.is_empty() {
        return Err(format!("{label} returned no bytes"));
    }
    Ok(bytes)
}

/// Download the release asset and its checksum sidecar, verify, and stage the
/// asset in the update directory. Returns the staged file's path.
#[tauri::command]
pub async fn download_update(
    app: tauri::AppHandle,
    download_url: String,
    checksum_url: String,
    asset_name: String,
    on_progress: Channel<UpdateProgress>,
) -> Result<String, String> {
    validate_asset_name(&asset_name)?;
    let download_url = validate_download_url(&download_url, "update download URL")?;
    let checksum_url = validate_download_url(&checksum_url, "checksum download URL")?;
    let update_root = update_dir(&app)?;
    fs::create_dir_all(&update_root).map_err(|e| e.to_string())?;

    let client = reqwest::Client::builder()
        .user_agent("quad-updater")
        .build()
        .map_err(|e| e.to_string())?;
    let package = fetch_bytes(
        &client,
        &download_url,
        "update package",
        MAX_PACKAGE_BYTES,
        Some(&on_progress),
    )
    .await?;
    let size = package.len() as u64;
    send_progress(&on_progress, "verifying", size, Some(size));
    let checksum = fetch_bytes(&client, &checksum_url, "checksum", MAX_CHECKSUM_BYTES, None).await?;
    let checksum_text =
        String::from_utf8(checksum).map_err(|_| "checksum file is not valid UTF-8".to_string())?;
    verify_checksum(&package, &checksum_text, &asset_name)?;

    let staged = update_root.join(&asset_name);
    fs::write(&staged, package).map_err(|e| e.to_string())?;
    Ok(staged.to_string_lossy().into_owned())
}

/// Resolve `path` and require it to live inside the update directory with an
/// expected file name — the path came from the webview.
fn canonical_staged_path(
    app: &tauri::AppHandle,
    path: &str,
    valid_name: fn(&str) -> bool,
) -> Result<PathBuf, String> {
    let root = fs::canonicalize(update_dir(app)?).map_err(|e| e.to_string())?;
    let staged = fs::canonicalize(Path::new(path)).map_err(|e| e.to_string())?;
    if !staged.starts_with(&root) {
        return Err("update file must be inside the update directory".into());
    }
    let name = staged
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("invalid update file path")?;
    if !valid_name(&name.to_ascii_lowercase()) {
        return Err("unsupported update file type".into());
    }
    Ok(staged)
}

/// macOS: unpack the staged `.app.zip` next to itself and return the `.app`
/// path. `ditto` preserves the code signature; the zip never carried a
/// quarantine attribute because the app itself downloaded it.
#[cfg(target_os = "macos")]
#[tauri::command]
pub fn extract_app_zip(app: tauri::AppHandle, zip_path: String) -> Result<String, String> {
    let zip = canonical_staged_path(&app, &zip_path, |n| n.ends_with(".app.zip"))?;
    let parent = zip.parent().ok_or("invalid zip path")?.to_path_buf();
    let app_path = parent.join("Quad.app");
    if app_path.exists() {
        fs::remove_dir_all(&app_path).map_err(|e| e.to_string())?;
    }
    let status = std::process::Command::new("ditto")
        .arg("-xk")
        .arg(zip.as_os_str())
        .arg(parent.as_os_str())
        .status()
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err("failed to extract update".into());
    }
    if !app_path.exists() {
        return Err("Quad.app not found in the update archive".into());
    }
    let _ = fs::remove_file(zip);
    Ok(app_path.to_string_lossy().into_owned())
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub fn extract_app_zip(_zip_path: String) -> Result<String, String> {
    Err("only available on macOS".into())
}

#[cfg(target_os = "windows")]
fn escape_powershell_literal(value: &str) -> String {
    value.replace('\'', "''")
}

/// Swap script for the Windows exe: wait for this process to exit, back up
/// the running exe, move the staged one into place, and relaunch. Retries
/// cover the window where the OS still holds the file; a run that never
/// succeeds retries elevated once, then restores the backup and leaves the
/// recovery marker.
#[cfg(target_os = "windows")]
fn windows_swap_script(
    pid: u32,
    source: &Path,
    target: &Path,
    log: &Path,
    error_marker: &Path,
    script: &Path,
) -> String {
    let template = r#"
param([switch]$Elevated)
$ErrorActionPreference = 'Stop'
$procId = __PID__
$source = '__SOURCE__'
$target = '__TARGET__'
$backup = "$target.previous"
$staged = "$target.update"
$log = '__LOG__'
$errorMarker = '__ERROR_MARKER__'
$script = '__SCRIPT__'

function Write-UpdateLog([string]$message) {
  try { Add-Content -LiteralPath $log -Value "$((Get-Date).ToString('s')) $message" } catch {}
}

Write-UpdateLog "Waiting for process $procId to exit."
while (Get-Process -Id $procId -ErrorAction SilentlyContinue) {
  Start-Sleep -Milliseconds 200
}

for ($attempt = 1; $attempt -le 120; $attempt++) {
  try {
    if (Test-Path -LiteralPath $backup) {
      Remove-Item -LiteralPath $backup -Force -ErrorAction SilentlyContinue
    }
    if (-not (Test-Path -LiteralPath $source)) {
      throw "Downloaded update is missing: $source"
    }
    Copy-Item -LiteralPath $source -Destination $staged -Force
    Move-Item -LiteralPath $target -Destination $backup -Force
    Move-Item -LiteralPath $staged -Destination $target -Force
    Write-UpdateLog "Installed update on attempt $attempt."
    Start-Process -FilePath $target
    Start-Sleep -Seconds 2
    Remove-Item -LiteralPath $backup -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $source -Force -ErrorAction SilentlyContinue
    exit 0
  } catch {
    Write-UpdateLog "Attempt $attempt failed: $($_.Exception.Message)"
    if ((-not (Test-Path -LiteralPath $target)) -and (Test-Path -LiteralPath $backup)) {
      try { Move-Item -LiteralPath $backup -Destination $target -Force } catch {}
    }
    Start-Sleep -Milliseconds 500
  }
}

if (-not $Elevated) {
  try {
    Write-UpdateLog "Retrying update with elevation."
    Start-Process -FilePath "powershell.exe" -Verb RunAs -ArgumentList @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $script, "-Elevated")
    exit 0
  } catch {
    Write-UpdateLog "Elevation failed: $($_.Exception.Message)"
  }
}

Write-UpdateLog "Update failed after retries."
try {
  Set-Content -LiteralPath $errorMarker -Value "The last update failed after Quad closed; the previous version was restored. See $log for details."
} catch {}
try {
  if ((-not (Test-Path -LiteralPath $target)) -and (Test-Path -LiteralPath $backup)) {
    Move-Item -LiteralPath $backup -Destination $target -Force
  }
  if (Test-Path -LiteralPath $target) { Start-Process -FilePath $target }
} catch {}
exit 1
"#;
    template
        .replace("__PID__", &pid.to_string())
        .replace("__SOURCE__", &escape_powershell_literal(&source.to_string_lossy()))
        .replace("__TARGET__", &escape_powershell_literal(&target.to_string_lossy()))
        .replace("__LOG__", &escape_powershell_literal(&log.to_string_lossy()))
        .replace(
            "__ERROR_MARKER__",
            &escape_powershell_literal(&error_marker.to_string_lossy()),
        )
        .replace("__SCRIPT__", &escape_powershell_literal(&script.to_string_lossy()))
}

#[cfg(target_os = "macos")]
fn escape_bash_literal(value: &str) -> String {
    value.replace('\'', "'\\''")
}

/// Spawn the platform swap script against the staged update and exit the app;
/// the script relaunches the new version.
#[tauri::command]
pub fn apply_update(app: tauri::AppHandle, update_path: String) -> Result<(), String> {
    if cfg!(debug_assertions) {
        return Err("self-update is disabled in dev builds".into());
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let _ = (app, update_path);
        Err("self-update is not supported on this platform".into())
    }

    #[cfg(any(target_os = "windows", target_os = "macos"))]
    {
        let current_exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let pid = std::process::id();
        let update_root = update_dir(&app)?;
        fs::create_dir_all(&update_root).map_err(|e| e.to_string())?;
        let error_marker = update_root.join(RECOVERY_ERROR_NAME);

        #[cfg(target_os = "windows")]
        {
            let staged = canonical_staged_path(&app, &update_path, |n| n.ends_with(".exe"))?;
            let log = update_root.join("install.log");
            let script_path = update_root.join("apply-update.ps1");
            let script =
                windows_swap_script(pid, &staged, &current_exe, &log, &error_marker, &script_path);
            fs::write(&script_path, script).map_err(|e| e.to_string())?;
            let mut cmd = std::process::Command::new("powershell.exe");
            cmd.args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-WindowStyle",
                "Hidden",
                "-File",
                &script_path.to_string_lossy(),
            ]);
            {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
            }
            cmd.spawn().map_err(|e| e.to_string())?;
        }

        #[cfg(target_os = "macos")]
        {
            let staged = canonical_staged_path(&app, &update_path, |n| n.ends_with(".app"))?;
            let app_bundle = current_exe
                .parent() // MacOS/
                .and_then(Path::parent) // Contents/
                .and_then(Path::parent) // Quad.app
                .ok_or("could not determine app bundle path")?;
            let backup = app_bundle.with_extension("app.previous");
            let script = format!(
                r#"set -e
pid={}
source='{}'
target='{}'
backup='{}'
error_marker='{}'
while kill -0 "$pid" 2>/dev/null; do sleep 0.2; done
trap 'echo "The last update failed after Quad closed; the previous version was restored." > "$error_marker"; if [ ! -e "$target" ] && [ -e "$backup" ]; then mv "$backup" "$target"; open "$target"; fi' ERR
rm -rf "$backup"
mv "$target" "$backup"
mv "$source" "$target"
open "$target"
rm -rf "$backup"
"#,
                pid,
                escape_bash_literal(&staged.to_string_lossy()),
                escape_bash_literal(&app_bundle.to_string_lossy()),
                escape_bash_literal(&backup.to_string_lossy()),
                escape_bash_literal(&error_marker.to_string_lossy()),
            );
            std::process::Command::new("bash")
                .args(["-c", &script])
                .spawn()
                .map_err(|e| e.to_string())?;
        }

        app.exit(0);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_names_reject_paths_and_unknown_types() {
        assert!(validate_asset_name("quad.exe").is_ok());
        assert!(validate_asset_name("quad-macos-universal.app.zip").is_ok());
        assert!(validate_asset_name("../evil.exe").is_err());
        assert!(validate_asset_name("dir\\evil.exe").is_err());
        assert!(validate_asset_name(".hidden.exe").is_err());
        assert!(validate_asset_name("notes.txt").is_err());
        assert!(validate_asset_name("").is_err());
    }

    #[test]
    fn download_urls_must_be_github_https() {
        assert!(validate_download_url("https://github.com/x/y/releases/download/v1/a.exe", "url").is_ok());
        assert!(validate_download_url("https://api.github.com/repos/x/y/releases/assets/1", "url").is_ok());
        assert!(validate_download_url("http://github.com/a.exe", "url").is_err());
        assert!(validate_download_url("https://evil.com/a.exe", "url").is_err());
    }

    #[test]
    fn checksum_parsing_handles_common_layouts() {
        let name = "quad.exe";
        let hash = "a".repeat(64);
        assert_eq!(
            expected_sha256(&format!("{hash}  {name}\n"), name),
            Some(hash.clone())
        );
        let aggregate = format!("{}  other.dmg\n{hash}  {name}\n", "b".repeat(64));
        assert_eq!(expected_sha256(&aggregate, name), Some(hash.clone()));
        assert_eq!(expected_sha256(&format!("{hash}\n"), name), Some(hash));
        assert_eq!(expected_sha256("no hashes here", name), None);
    }

    #[test]
    fn checksum_verification_matches_sha256() {
        let name = "quad.exe";
        let bytes = b"quad update bytes";
        let hash = format!("{:x}", Sha256::digest(bytes));
        assert!(verify_checksum(bytes, &format!("{hash}  {name}"), name).is_ok());
        assert!(verify_checksum(bytes, &format!("{}  {name}", "0".repeat(64)), name).is_err());
    }
}
