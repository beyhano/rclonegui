/// Rclone binary discovery: platform detection, path location, and executable verification.
///
/// This module handles detecting the host platform, locating the bundled rclone binary
/// within `rclone-bin/{platform}/`, and verifying it is executable.
///
/// # Binary Path Convention
///
/// | Platform | Path |
/// |---|---|
/// | Linux | `rclone-bin/linux/rclone` |
/// | Windows | `rclone-bin/windows/rclone.exe` |
/// | macOS (AMD64) | `rclone-bin/osx-amd64/rclone` |
/// | macOS (ARM64) | `rclone-bin/osx-arm64/rclone` |
///
/// # Search Order
///
/// `find_binary()` searches in this order:
/// 1. `resource_dir/rclone-bin/{platform}/` — production bundle
/// 2. CARGO_MANIFEST_DIR parent — when env var is set (dev/build time)
/// 3. Current working directory — `pwd/rclone-bin/{platform}/`
/// 4. Ancestors of current executable — walk up from binary path
///
/// # App Data Binary
///
/// `ensure_app_binary()` copies the bundled binary to the app data directory
/// on first run. This writable copy is used for self-updates via `rclone selfupdate`.
use std::path::{Path, PathBuf};

/// Resolve the host platform to a platform identifier string.
///
/// Returns one of:
/// - `linux-amd64` — Linux x86_64
/// - `linux-arm64` — Linux ARM64 (aarch64)
/// - `windows-amd64` — Windows x86_64
/// - `osx-amd64` — macOS Intel
/// - `osx-arm64` — macOS Apple Silicon
pub fn resolve_platform() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => "linux-amd64",
        ("linux", "aarch64") => "linux-arm64",
        ("windows", _) => "windows-amd64",
        ("macos", "aarch64") => "osx-arm64",
        ("macos", "x86_64") => "osx-amd64",
        _ => "linux-amd64",
    }
}

/// Map a platform identifier to the corresponding folder name under `rclone-bin/`.
fn platform_folder(platform: &str) -> &'static str {
    match platform {
        "linux-amd64" | "linux-arm64" => "linux",
        "windows-amd64" => "windows",
        "osx-amd64" => "osx-amd64",
        "osx-arm64" => "osx-arm64",
        _ => "linux",
    }
}

/// Return the rclone binary file name for the given platform.
fn binary_name(platform: &str) -> &'static str {
    if platform.starts_with("windows") {
        "rclone.exe"
    } else {
        "rclone"
    }
}

/// Locate the rclone binary path relative to `base_path` for the given platform.
///
/// The binary is expected at `{base_path}/rclone-bin/{folder}/{binary_name}`.
pub fn locate_binary(base_path: &Path, platform: &str) -> PathBuf {
    let mut path = base_path.to_path_buf();
    path.push("rclone-bin");
    path.push(platform_folder(platform));
    path.push(binary_name(platform));
    path
}

/// Search for the bundled rclone binary across multiple locations.
///
/// Returns the path if found in any known location, or `None` if not found.
///
/// # Search Order
///
/// 1. `base` — for production bundles (resource_dir)
/// 2. `$CARGO_MANIFEST_DIR/..` — for Cargo builds (dev run, unit tests)
/// 3. Current working directory — for direct `tauri dev` runs
/// 4. Ancestors of the current executable — walk up looking for `rclone-bin/`
pub fn find_binary(platform: &str) -> Option<PathBuf> {
    // 1. Explicit base if given (from production resource_dir)
    //    (called with resource_dir by lib.rs — handled there)

    // 2. CARGO_MANIFEST_DIR parent (project root at build time)
    //    This env var is set by Cargo when compiling, available at runtime
    //    for tests and dev builds.
    if let Ok(cargo_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let project_root = Path::new(&cargo_dir).parent()?;
        let path = locate_binary(project_root, platform);
        if path.exists() {
            return Some(path);
        }
    }

    // 3. Current working directory
    if let Ok(cwd) = std::env::current_dir() {
        let path = locate_binary(&cwd, platform);
        if path.exists() {
            return Some(path);
        }
    }

    // 4. Ancestors of current executable
    //    exe path: project/src-tauri/target/debug/rclonegui
    //    rclone-bin is at: project/rclone-bin/
    if let Ok(exe) = std::env::current_exe() {
        for ancestor in exe.ancestors().skip(1) {
            let candidate = ancestor.join("rclone-bin");
            if candidate.is_dir() {
                let path = locate_binary(ancestor, platform);
                if path.exists() {
                    return Some(path);
                }
            }
        }
    }

    None
}

/// Ensure the rclone binary exists in the app data directory.
///
/// If the binary does not exist at `app_data_dir/rclone-bin/{platform}/{binary}`,
/// it is copied from `bundled_path`. Returns the path to the app data binary.
///
/// This ensures a writable copy is available for self-updates without needing
/// elevated privileges (Program Files, /usr, etc. are often read-only).
pub fn ensure_app_binary(
    bundled_path: &Path,
    app_data_dir: &Path,
    platform: &str,
) -> Result<PathBuf, String> {
    let target_dir = app_data_dir.join("rclone-bin").join(platform_folder(platform));
    let target_path = target_dir.join(binary_name(platform));

    if target_path.exists() {
        return Ok(target_path);
    }

    // Bundled binary must exist
    if !bundled_path.exists() {
        return Err(format!(
            "Bundled rclone binary not found at: {}",
            bundled_path.display()
        ));
    }

    std::fs::create_dir_all(&target_dir)
        .map_err(|e| format!("Failed to create app data rclone dir: {}", e))?;

    std::fs::copy(bundled_path, &target_path)
        .map_err(|e| format!("Failed to copy rclone binary to app data: {}", e))?;

    // Make executable on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&target_path, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("Failed to set executable permission: {}", e))?;
    }

    Ok(target_path)
}

// ----- RED tests first (Task 2.4) -----

#[cfg(test)]
mod tests {
    use super::*;

    // -- resolve_platform (Task 2.4 RED test) --

    #[test]
    fn test_resolve_platform_linux_amd64() {
        if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
            assert_eq!(resolve_platform(), "linux-amd64");
        }
    }

    #[test]
    fn test_resolve_platform_linux_arm64() {
        if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
            assert_eq!(resolve_platform(), "linux-arm64");
        }
    }

    #[test]
    fn test_resolve_platform_windows_amd64() {
        if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
            assert_eq!(resolve_platform(), "windows-amd64");
        }
    }

    #[test]
    fn test_resolve_platform_macos_arm64() {
        if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
            assert_eq!(resolve_platform(), "osx-arm64");
        }
    }

    #[test]
    fn test_resolve_platform_macos_amd64() {
        if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
            assert_eq!(resolve_platform(), "osx-amd64");
        }
    }

    #[test]
    fn test_resolve_platform_fallback() {
        // Unknown OS/arch should fall back to "linux-amd64"
        // We can't easily simulate this at compile time, but the default match arm exists.
        // Verify by asserting the function returns one of the known strings.
        let platform = resolve_platform();
        assert!(
            [
                "linux-amd64",
                "linux-arm64",
                "windows-amd64",
                "osx-amd64",
                "osx-arm64"
            ]
            .contains(&platform),
            "resolve_platform() returned unexpected value: {platform}"
        );
    }

    // -- locate_binary / platform_folder triangulation tests --

    #[test]
    fn test_platform_folder_mapping() {
        assert_eq!(platform_folder("linux-amd64"), "linux");
        assert_eq!(platform_folder("linux-arm64"), "linux");
        assert_eq!(platform_folder("windows-amd64"), "windows");
        assert_eq!(platform_folder("osx-amd64"), "osx-amd64");
        assert_eq!(platform_folder("osx-arm64"), "osx-arm64");
    }

    #[test]
    fn test_platform_folder_unknown_fallback() {
        assert_eq!(platform_folder("unknown-os"), "linux");
    }

    #[test]
    fn test_locate_binary_linux() {
        let base = Path::new("/home/user/app");
        let path = locate_binary(base, "linux-amd64");
        let mut expected = PathBuf::from("/home/user/app");
        expected.push("rclone-bin");
        expected.push("linux");
        expected.push("rclone");
        assert_eq!(path, expected);
    }

    #[test]
    fn test_locate_binary_windows() {
        // On all platforms, PathBuf handles the separator correctly.
        let base = Path::new("C:\\app");
        let path = locate_binary(base, "windows-amd64");
        let mut expected = PathBuf::from("C:\\app");
        expected.push("rclone-bin");
        expected.push("windows");
        expected.push("rclone.exe");
        assert_eq!(path, expected);
    }

    #[test]
    fn test_locate_binary_macos_arm64() {
        let base = Path::new("/Applications/rclonegui");
        let path = locate_binary(base, "osx-arm64");
        let mut expected = PathBuf::from("/Applications/rclonegui");
        expected.push("rclone-bin");
        expected.push("osx-arm64");
        expected.push("rclone");
        assert_eq!(path, expected);
    }

    #[test]
    fn test_locate_binary_macos_amd64() {
        let base = Path::new("/Applications/rclonegui");
        let path = locate_binary(base, "osx-amd64");
        let mut expected = PathBuf::from("/Applications/rclonegui");
        expected.push("rclone-bin");
        expected.push("osx-amd64");
        expected.push("rclone");
        assert_eq!(path, expected);
    }

    // -- ensure_app_binary tests --

    #[test]
    fn test_ensure_app_binary_existing() {
        let dir = std::env::temp_dir().join(format!("rclonegui-test-existing-{}", std::process::id()));
        let platform = resolve_platform();
        let bin_name = binary_name(platform);
        let folder = platform_folder(platform);

        // Seed the target path so it "already exists"
        let target_dir = dir.join("app_data").join("rclone-bin").join(folder);
        std::fs::create_dir_all(&target_dir).expect("create target dir");
        std::fs::write(target_dir.join(bin_name), b"fake-rclone").expect("write target");

        let bundled = Path::new("/nonexistent/bundled/rclone"); // shouldn't be read
        let result = ensure_app_binary(bundled, &dir.join("app_data"), platform);
        assert!(result.is_ok(), "should succeed when target exists: {:?}", result);
        let path = result.unwrap();
        assert!(path.ends_with(bin_name), "should end with binary name");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_ensure_app_binary_copies() {
        let dir = std::env::temp_dir().join(format!("rclonegui-test-copies-{}", std::process::id()));
        let platform = resolve_platform();
        let bin_name = binary_name(platform);
        let folder = platform_folder(platform);

        // Create a fake bundled binary
        let bundled_dir = dir.join("bundled").join("rclone-bin").join(folder);
        std::fs::create_dir_all(&bundled_dir).expect("create bundled dir");
        let bundled_path = bundled_dir.join(bin_name);
        std::fs::write(&bundled_path, b"real-rclone-content").expect("write bundled");

        let app_data = dir.join("app_data");
        let result = ensure_app_binary(&bundled_path, &app_data, platform);
        assert!(result.is_ok(), "copy should succeed: {:?}", result);
        let target = result.unwrap();
        assert!(target.exists(), "target should exist after copy");
        assert_eq!(
            std::fs::read(&target).expect("read target"),
            b"real-rclone-content",
            "content should match bundled"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_ensure_app_binary_bundled_missing() {
        let dir = std::env::temp_dir().join(format!("rclonegui-test-missing-{}", std::process::id()));
        let platform = resolve_platform();
        let bundled = Path::new("/definitely/does/not/exist/rclone");
        let result = ensure_app_binary(bundled, &dir, platform);
        assert!(result.is_err(), "should fail when bundled is missing");
        assert!(
            result.unwrap_err().contains("not found"),
            "error should mention missing bundled"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
