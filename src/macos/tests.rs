use crate::{
    macos::{percent_encode, DeleteMethod, TrashContextExtMacos},
    tests::{get_unique_name, init_logging},
    TrashContext,
};
use serial_test::serial;
use std::ffi::OsStr;
use std::fs::File;
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;
use std::process::Command;

/// Holds a list of paths to files to clean up after a test.
///
/// Simply push the paths for whatever file was created during tests and the
/// `Drop` implementation will clean up the files after the test.
struct CleanupPaths(Vec<PathBuf>);

impl CleanupPaths {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn push(&mut self, path: PathBuf) {
        self.0.push(path);
    }
}

impl Drop for CleanupPaths {
    fn drop(&mut self) {
        for path in &self.0 {
            let _ = std::fs::remove_file(path);
        }
    }
}

#[test]
#[serial]
fn test_delete_with_finder_quoted_paths() {
    init_logging();
    let mut trash_ctx = TrashContext::default();
    trash_ctx.set_delete_method(DeleteMethod::Finder);

    let mut path1 = PathBuf::from(get_unique_name());
    let mut path2 = PathBuf::from(get_unique_name());
    path1.set_extension(r#"a"b,"#);
    path2.set_extension(r#"x80=%80 slash=\ pc=% quote=" comma=,"#);
    File::create_new(&path1).unwrap();
    File::create_new(&path2).unwrap();
    trash_ctx.delete_all(&[&path1, &path2]).unwrap();
    assert!(!path1.exists());
    assert!(!path2.exists());
}

#[test]
#[serial]
fn test_delete_with_ns_file_manager() {
    init_logging();
    let mut trash_ctx = TrashContext::default();
    trash_ctx.set_delete_method(DeleteMethod::NsFileManager);

    let path = get_unique_name();
    File::create_new(&path).unwrap();
    trash_ctx.delete(&path).unwrap();
    assert!(File::open(&path).is_err());
}

#[test]
#[serial]
fn test_delete_binary_path_with_ns_file_manager() {
    let (_cleanup, tmp) = create_hfs_volume().unwrap();
    let parent_fs_supports_binary = tmp.path();

    init_logging();
    for method in [DeleteMethod::NsFileManager, DeleteMethod::Finder] {
        let mut trash_ctx = TrashContext::default();
        trash_ctx.set_delete_method(method);

        let mut path_invalid = parent_fs_supports_binary.join(get_unique_name());
        path_invalid.set_extension(OsStr::from_bytes(b"\x80\"\\")); //...trash-test-111-0.\x80 (not push to avoid fail unexisting dir)

        File::create_new(&path_invalid).unwrap();

        assert!(path_invalid.exists());
        trash_ctx.delete(&path_invalid).unwrap();
        assert!(!path_invalid.exists());
    }
}

#[test]
fn test_path_byte() {
    let invalid_utf8 = b"\x80"; // lone continuation byte (128) (invalid utf8)
    let percent_encoded = "%80"; // valid macOS path in a %-escaped encoding

    let mut expected_path = PathBuf::from(get_unique_name());
    let mut path_with_invalid_utf8 = expected_path.clone();

    path_with_invalid_utf8.push(OsStr::from_bytes(invalid_utf8)); //      trash-test-111-0/\x80
    expected_path.push(percent_encoded); //                    trash-test-111-0/%80

    let actual = percent_encode(&path_with_invalid_utf8.as_os_str().as_encoded_bytes()); // trash-test-111-0/%80
    assert_eq!(std::path::Path::new(actual.as_ref()), expected_path);
}

fn create_hfs_volume() -> std::io::Result<(impl Drop, tempfile::TempDir)> {
    let tmp = tempfile::tempdir()?;
    let dmg_file = tmp.path().join("fs.dmg");
    let cleanup = {
        // Create dmg file
        Command::new("hdiutil").args(["create", "-size", "1m", "-fs", "HFS+"]).arg(&dmg_file).status()?;

        // Mount dmg file into temporary location
        Command::new("hdiutil").args(["attach", "-nobrowse", "-mountpoint"]).arg(tmp.path()).arg(&dmg_file).status()?;

        // Ensure that the mount point is always cleaned up
        defer::defer({
            let mount_point = tmp.path().to_owned();
            move || {
                Command::new("hdiutil")
                    .arg("detach")
                    .arg(&mount_point)
                    .status()
                    .expect("detach temporary test dmg filesystem successfully");
            }
        })
    };
    Ok((cleanup, tmp))
}

#[test]
#[serial]
fn test_delete_with_info_ns_file_manager() {
    let mut cleanup_paths = CleanupPaths::new();
    let path = std::env::current_dir().expect("Should be able to get current directory").join(get_unique_name());
    cleanup_paths.push(path.clone());
    File::create_new(&path).unwrap();

    let mut trash = TrashContext::new();
    trash.set_delete_method(DeleteMethod::NsFileManager);

    match trash.delete_with_info(&path) {
        Ok(trash_item) => {
            let id_path = PathBuf::from(&trash_item.id);
            cleanup_paths.push(id_path.clone());

            assert_eq!(trash_item.name, path.components().last().expect("Should have last component").as_os_str());
            assert_eq!(trash_item.original_parent, path.parent().expect("Should have parent").as_os_str());
            assert!(id_path.to_string_lossy().contains(".Trash"))
        }
        _ => panic!("Calling delete_with_info failed to return TrashItem."),
    }
}

#[test]
#[serial]
fn test_delete_with_info_finder() {
    let mut cleanup_paths = CleanupPaths::new();
    let path = std::env::current_dir().expect("Should be able to get current directory").join(get_unique_name());
    cleanup_paths.push(path.clone());
    File::create_new(&path).unwrap();

    let mut trash = TrashContext::new();
    trash.set_delete_method(DeleteMethod::Finder);

    match trash.delete_with_info(&path) {
        Ok(trash_item) => {
            let id_path = PathBuf::from(&trash_item.id);
            cleanup_paths.push(id_path.clone());

            assert_eq!(trash_item.name, path.components().last().expect("Should have last component").as_os_str());
            assert_eq!(trash_item.original_parent, path.parent().expect("Should have parent").as_os_str());
            assert!(id_path.to_string_lossy().contains(".Trash"))
        }
        _ => panic!("Calling delete_with_info with Finder method failed to return TrashItem."),
    }
}
