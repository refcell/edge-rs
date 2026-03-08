//! Unit tests for the edgeup-lib crate.

use crate::Installer;

#[test]
fn test_installer_with_dir_creates_empty() {
    let dir = std::env::temp_dir().join("edgeup-lib-test-new");
    let _ = std::fs::remove_dir_all(&dir);
    let installer = Installer::with_dir(dir.clone());
    // Just check we can create it without error.
    drop(installer);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_list_installed_empty() {
    // When versions dir doesn't exist, should return empty vec.
    let dir = std::env::temp_dir().join("edgeup-lib-test-list-empty");
    let _ = std::fs::remove_dir_all(&dir);
    let installer = Installer::with_dir(dir.clone());
    let versions = installer.list_installed_versions().unwrap();
    assert!(versions.is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_resolve_version_installed() {
    // If a version binary exists on disk, resolve_version returns its path.
    let dir = std::env::temp_dir().join("edgeup-lib-test-resolve");
    let _ = std::fs::remove_dir_all(&dir);
    let installer = Installer::with_dir(dir.clone());

    // Manually create a fake binary.
    let version_dir = dir.join("versions").join("v0.0.1-test");
    std::fs::create_dir_all(&version_dir).unwrap();
    let binary = version_dir.join("edgec");
    std::fs::write(&binary, b"fake binary").unwrap();

    let path = installer.resolve_version("v0.0.1-test").unwrap();
    assert_eq!(path, binary);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_list_installed_with_versions() {
    let dir = std::env::temp_dir().join("edgeup-lib-test-list");
    let _ = std::fs::remove_dir_all(&dir);
    let installer = Installer::with_dir(dir.clone());

    // Create fake version dirs.
    for v in &["v0.1.0", "v0.1.1", "v0.2.0"] {
        let vdir = dir.join("versions").join(v);
        std::fs::create_dir_all(&vdir).unwrap();
        std::fs::write(vdir.join("edgec"), b"fake").unwrap();
    }

    let mut versions = installer.list_installed_versions().unwrap();
    versions.sort();
    assert_eq!(versions, vec!["v0.1.0", "v0.1.1", "v0.2.0"]);

    let _ = std::fs::remove_dir_all(&dir);
}
