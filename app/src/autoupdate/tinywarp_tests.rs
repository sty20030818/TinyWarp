use super::*;

fn valid() -> LocalRelease {
    LocalRelease {
        version: "v0.1.0-dev.1".into(),
        sha256: "a".repeat(64),
        architecture: std::env::consts::ARCH.into(),
    }
}

#[test]
fn accepts_only_valid_version_architecture_and_digest() {
    let mut release = valid();
    assert!(validate_release(&release).is_ok());
    release.version = "../../escape".into();
    assert!(validate_release(&release).is_err());
    release = valid();
    release.sha256 = "z".repeat(64);
    assert!(validate_release(&release).is_err());
    release = valid();
    release.architecture = "wrong".into();
    assert!(validate_release(&release).is_err());
}

#[test]
fn rejects_symlinks_and_shared_permissions() {
    let directory = tempfile::tempdir().unwrap();
    fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o700)).unwrap();
    assert!(trusted_path(directory.path()).is_ok());
    let link = directory.path().join("link");
    std::os::unix::fs::symlink(directory.path(), &link).unwrap();
    assert!(trusted_path(&link).is_err());
    fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o755)).unwrap();
    assert!(trusted_path(directory.path()).is_err());
}

#[test]
fn exchange_preserves_complete_old_bundle_and_is_reversible_with_spaces() {
    let directory = tempfile::tempdir().unwrap();
    let old = directory.path().join("Tiny Warp 中文.app");
    let new = directory.path().join("backup new.app");
    fs::create_dir(&old).unwrap();
    fs::create_dir(&new).unwrap();
    fs::write(old.join("version"), "old").unwrap();
    fs::write(new.join("version"), "new").unwrap();
    exchange(&old, &new).unwrap();
    assert_eq!(fs::read_to_string(old.join("version")).unwrap(), "new");
    assert_eq!(fs::read_to_string(new.join("version")).unwrap(), "old");
    exchange(&old, &new).unwrap();
    assert_eq!(fs::read_to_string(old.join("version")).unwrap(), "old");
    let missing = directory.path().join("missing");
    assert!(exchange(&old, &missing).is_err());
    assert_eq!(fs::read_to_string(old.join("version")).unwrap(), "old");
}

#[test]
fn archive_checksum_covers_file_contents() {
    let file = tempfile::NamedTempFile::new().unwrap();
    fs::write(file.path(), "abc").unwrap();
    assert_eq!(
        sha256(file.path()).unwrap(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    fs::write(file.path(), "abd").unwrap();
    assert_ne!(
        sha256(file.path()).unwrap(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn retired_local_inbox_is_ignored_but_invalid_new_releases_fail() {
    let mut release = valid();
    release.version = "v0.2026.11.12.00.30.tinywarp_02".into();
    let old = serde_json::to_vec(&release).unwrap();
    assert!(decode_release(old.as_slice()).unwrap().is_none());
    release.version = "v0.1.0".into();
    assert!(
        decode_release(serde_json::to_vec(&release).unwrap().as_slice())
            .unwrap()
            .is_some()
    );
    release.version = "v0.01.0".into();
    assert!(decode_release(serde_json::to_vec(&release).unwrap().as_slice()).is_err());
}
