use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

const REPO: &str = "aha-app/overitall";

/// Construct asset name for given OS and architecture
fn asset_name_for_platform(os: &str, arch: &str) -> anyhow::Result<String> {
    let os_name = match os {
        "macos" => "macos",
        "linux" => "linux",
        other => anyhow::bail!("Unsupported OS: {}", other),
    };
    let arch_name = match arch {
        "aarch64" => "arm64",
        "x86_64" => "x86_64",
        other => anyhow::bail!("Unsupported architecture: {}", other),
    };
    Ok(format!("oit-{}-{}.tar.gz", os_name, arch_name))
}

/// Get the asset name for the current platform
fn asset_name() -> anyhow::Result<String> {
    asset_name_for_platform(std::env::consts::OS, std::env::consts::ARCH)
}

/// Get latest release version from GitHub API
fn get_latest_version() -> anyhow::Result<String> {
    let url = format!("https://api.github.com/repos/{}/releases/latest", REPO);
    let response: serde_json::Value = ureq::get(&url)
        .set("User-Agent", "oit-updater")
        .call()
        .map_err(|e| anyhow::anyhow!("Failed to check for updates: {}", e))?
        .into_json()?;

    let tag = response["tag_name"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No tag_name in response"))?;

    Ok(tag.trim_start_matches('v').to_string())
}

/// Download and extract release with direct HTTP
fn download_and_update(tag: &str) -> anyhow::Result<()> {
    let asset_name = asset_name()?;
    let temp_dir = tempfile::tempdir()?;
    let temp_path = temp_dir.path();
    let asset_path = temp_path.join(&asset_name);

    let url = format!(
        "https://github.com/{}/releases/download/{}/{}",
        REPO, tag, asset_name
    );

    println!("Downloading {}...", asset_name);

    let response = ureq::get(&url)
        .set("User-Agent", "oit-updater")
        .call()
        .map_err(|e| anyhow::anyhow!("Failed to download release: {}", e))?;

    let mut file = fs::File::create(&asset_path)?;
    let mut reader = response.into_reader();
    let mut buffer = [0u8; 8192];
    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        file.write_all(&buffer[..bytes_read])?;
    }
    drop(file);

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

    fs::remove_file(&current_exe)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asset_name_for_macos_arm64() {
        let result = asset_name_for_platform("macos", "aarch64");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "oit-macos-arm64.tar.gz");
    }

    #[test]
    fn test_asset_name_for_macos_x86_64() {
        let result = asset_name_for_platform("macos", "x86_64");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "oit-macos-x86_64.tar.gz");
    }

    #[test]
    fn test_asset_name_for_linux_arm64() {
        let result = asset_name_for_platform("linux", "aarch64");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "oit-linux-arm64.tar.gz");
    }

    #[test]
    fn test_asset_name_for_linux_x86_64() {
        let result = asset_name_for_platform("linux", "x86_64");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "oit-linux-x86_64.tar.gz");
    }

    #[test]
    fn test_asset_name_for_unsupported_os() {
        let result = asset_name_for_platform("windows", "x86_64");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported OS: windows"));
    }

    #[test]
    fn test_asset_name_for_unsupported_arch() {
        let result = asset_name_for_platform("linux", "i686");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported architecture: i686"));
    }

    #[test]
    fn test_asset_name_for_current_platform() {
        // Test that the function works for the current platform
        let result = asset_name();
        assert!(result.is_ok(), "asset_name() should work on the current platform");

        let asset = result.unwrap();
        assert!(asset.starts_with("oit-"), "Asset name should start with 'oit-'");
        assert!(asset.ends_with(".tar.gz"), "Asset name should end with '.tar.gz'");

        // Verify the format matches expected pattern
        assert!(
            asset == "oit-macos-arm64.tar.gz" ||
            asset == "oit-macos-x86_64.tar.gz" ||
            asset == "oit-linux-arm64.tar.gz" ||
            asset == "oit-linux-x86_64.tar.gz",
            "Asset name '{}' should be one of the supported formats", asset
        );
    }
}
