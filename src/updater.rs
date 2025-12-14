use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

const REPO: &str = "jemmyw/overitall";
const ASSET_NAME: &str = "oit-macos-arm64.tar.gz";

/// Check if gh CLI is available and authenticated
pub fn gh_is_available() -> bool {
    Command::new("gh")
        .args(["auth", "status"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Get latest release version using gh CLI
fn get_latest_version() -> anyhow::Result<String> {
    let output = Command::new("gh")
        .args([
            "release",
            "view",
            "--repo",
            REPO,
            "--json",
            "tagName",
            "-q",
            ".tagName",
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("gh release view failed: {}", stderr);
    }

    let version = String::from_utf8(output.stdout)?
        .trim()
        .trim_start_matches('v')
        .to_string();

    Ok(version)
}

/// Download and extract release using gh CLI with progress output
fn download_and_update(tag: &str) -> anyhow::Result<()> {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = tempfile::tempdir()?;
    let temp_path = temp_dir.path();
    let asset_path = temp_path.join(ASSET_NAME);

    println!("Downloading {}...", ASSET_NAME);

    let mut child = Command::new("gh")
        .args([
            "release",
            "download",
            tag,
            "--repo",
            REPO,
            "--pattern",
            ASSET_NAME,
            "--dir",
            temp_path.to_str().unwrap(),
        ])
        .stderr(Stdio::piped())
        .spawn()?;

    if let Some(stderr) = child.stderr.take() {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            if let Ok(line) = line {
                if !line.is_empty() {
                    println!("  {}", line);
                }
            }
        }
    }

    let status = child.wait()?;
    if !status.success() {
        anyhow::bail!("Failed to download release");
    }

    if !asset_path.exists() {
        anyhow::bail!("Downloaded file not found at {:?}", asset_path);
    }

    println!("Extracting...");

    let extract_status = Command::new("tar")
        .args(["-xzf", asset_path.to_str().unwrap()])
        .current_dir(temp_path)
        .status()?;

    if !extract_status.success() {
        anyhow::bail!("Failed to extract archive");
    }

    let new_binary = temp_path.join("oit");
    if !new_binary.exists() {
        anyhow::bail!("oit binary not found in archive");
    }

    println!("Installing...");

    let current_exe = std::env::current_exe()?;

    fs::copy(&new_binary, &current_exe)?;

    let mut perms = fs::metadata(&current_exe)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&current_exe, perms)?;

    Ok(())
}

/// Check for updates and apply if available.
/// If an update is applied, this function re-execs and never returns.
/// Returns Ok(()) if no update needed, Err if something went wrong.
pub fn check_and_update(current_version: &str) -> anyhow::Result<()> {
    if !gh_is_available() {
        anyhow::bail!(
            "gh CLI not found or not authenticated.\n\
             Install gh and run: gh auth login"
        );
    }

    let latest_version = get_latest_version()?;

    if latest_version == current_version {
        return Ok(());
    }

    println!("Updating oit: {} -> {}", current_version, latest_version);
    let tag = format!("v{}", latest_version);
    download_and_update(&tag)?;

    println!("Update complete! Restarting...\n");

    let exe = std::env::current_exe()?;
    let args: Vec<String> = std::env::args().skip(1).collect();

    let err = exec::execvp(
        &exe,
        &std::iter::once(exe.to_string_lossy().into_owned())
            .chain(args)
            .collect::<Vec<_>>(),
    );

    anyhow::bail!("Failed to restart: {}", err);
}
