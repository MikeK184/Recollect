//! Descriptor-relative exclusive private artifacts. No new Cargo dependency.
use anyhow::{Context, Result, ensure};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    ffi::CString,
    fs::{self, File},
    io::Write,
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::fs::MetadataExt,
    },
    path::{Component, Path},
    process::{Command, Stdio},
    sync::{Mutex, OnceLock},
};
#[cfg(target_os = "macos")]
const DIRECTORY: i32 = 0x100000;
#[cfg(not(target_os = "macos"))]
const DIRECTORY: i32 = 0x10000;
#[cfg(target_os = "macos")]
const NOFOLLOW: i32 = 0x100;
#[cfg(not(target_os = "macos"))]
const NOFOLLOW: i32 = 0x20000;
#[cfg(target_os = "macos")]
const CREATE_EXCL: i32 = 0x200 | 0x800;
#[cfg(not(target_os = "macos"))]
const CREATE_EXCL: i32 = 0x40 | 0x80;
#[cfg(target_os = "macos")]
const CLOEXEC: i32 = 0x1000000;
#[cfg(not(target_os = "macos"))]
const CLOEXEC: i32 = 0x80000;
unsafe extern "C" {
    fn geteuid() -> u32;
    fn openat(fd: i32, path: *const std::ffi::c_char, flags: i32, ...) -> i32;
    fn mkdirat(fd: i32, path: *const std::ffi::c_char, mode: u32) -> i32;
}
fn names(path: &std::ffi::OsStr) -> Result<CString> {
    use std::os::unix::ffi::OsStrExt;
    Ok(CString::new(path.as_bytes())?)
}
fn at(parent: &File, name: &std::ffi::OsStr, flags: i32, mode: u32) -> Result<File> {
    let c = names(name)?;
    let fd = unsafe { openat(parent.as_raw_fd(), c.as_ptr(), flags | CLOEXEC, mode) };
    if fd < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(unsafe { File::from_raw_fd(fd) })
}
fn owners() -> &'static Mutex<BTreeMap<std::path::PathBuf, File>> {
    static OWNED: OnceLock<Mutex<BTreeMap<std::path::PathBuf, File>>> = OnceLock::new();
    OWNED.get_or_init(|| Mutex::new(BTreeMap::new()))
}
fn same(a: &File, b: &File) -> Result<bool> {
    let a = a.metadata()?;
    let b = b.metadata()?;
    Ok(a.dev() == b.dev() && a.ino() == b.ino())
}
pub fn directory(path: &Path) -> Result<File> {
    ensure!(path.is_absolute(), "Absolute private path required");
    let mut fd = File::open("/")?;
    let mut current = std::path::PathBuf::from("/");
    for part in path.components() {
        match part {
            Component::RootDir => {}
            Component::Normal(n) => {
                fd = at(&fd, n, DIRECTORY | NOFOLLOW, 0)?;
                current.push(n);
                if let Some(owned) = owners()
                    .lock()
                    .map_err(|_| anyhow::anyhow!("Ownership guard unavailable"))?
                    .get(&current)
                {
                    ensure!(same(&fd, owned)?, "Owned directory replaced");
                }
            }
            _ => anyhow::bail!("Path traversal rejected"),
        }
    }
    Ok(fd)
}
pub fn new_directory(path: &Path) -> Result<()> {
    let parent = directory(path.parent().context("Output parent")?)?;
    let name = path.file_name().context("Output name")?;
    let c = names(name)?;
    ensure!(
        unsafe { mkdirat(parent.as_raw_fd(), c.as_ptr(), 0o700) } == 0,
        "Private directory creation failed"
    );
    let dir = at(&parent, name, DIRECTORY | NOFOLLOW, 0)?;
    let md = dir.metadata()?;
    ensure!(
        md.is_dir() && md.mode() & 0o777 == 0o700 && md.nlink() == 2,
        "New private directory identity invalid"
    );
    ensure!(
        fs::read_dir(path)?.next().is_none(),
        "New directory not empty"
    );
    ensure!(
        same(&dir, &at(&parent, name, DIRECTORY | NOFOLLOW, 0)?)?,
        "Directory name replaced"
    );
    owners()
        .lock()
        .map_err(|_| anyhow::anyhow!("Ownership guard unavailable"))?
        .insert(path.into(), dir);
    parent.sync_all()?;
    Ok(())
}
pub fn write(path: &Path, bytes: &[u8]) -> Result<()> {
    ensure!(bytes.len() <= 96 * 1024 * 1024, "Artifact exceeds cap");
    let parent_path = path.parent().context("Artifact parent")?;
    let fd = directory(parent_path)?;
    let mut f = at(
        &fd,
        path.file_name().context("Artifact name")?,
        1 | CREATE_EXCL | NOFOLLOW,
        0o600,
    )?;
    ensure!(
        f.metadata()?.is_file()
            && f.metadata()?.mode() & 0o777 == 0o600
            && f.metadata()?.nlink() == 1
            && f.metadata()?.uid() == unsafe { geteuid() },
        "Private artifact invalid"
    );
    f.write_all(bytes)?;
    f.sync_all()?;
    let current = at(&fd, path.file_name().context("Artifact name")?, NOFOLLOW, 0)?;
    ensure!(
        same(&f, &current)? && current.metadata()?.nlink() == 1,
        "Artifact identity changed"
    );
    fd.sync_all()?;
    ensure!(
        same(&fd, &directory(parent_path)?)?,
        "Directory name changed during write"
    );
    Ok(())
}
pub fn json(path: &Path, value: &Value) -> Result<()> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    write(path, &bytes)
}
pub fn sha(bytes: &[u8]) -> Result<String> {
    let mut child = Command::new("python3")
        .args([
            "-I",
            "-B",
            "-c",
            "import hashlib,sys;print(hashlib.sha256(sys.stdin.buffer.read()).hexdigest())",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    child.stdin.take().context("Hash input")?.write_all(bytes)?;
    let output = child.wait_with_output()?;
    ensure!(output.status.success(), "Hash failed");
    Ok(String::from_utf8(output.stdout)?.trim().into())
}
pub fn hash_file(path: &Path) -> Result<String> {
    use std::io::Read;
    let parent = directory(path.parent().context("Hash parent")?)?;
    let mut file = at(&parent, path.file_name().context("Hash name")?, NOFOLLOW, 0)?;
    let md = file.metadata()?;
    ensure!(
        md.is_file() && md.len() <= 512 * 1024 * 1024,
        "Hash input invalid"
    );
    let mut bytes = Vec::new();
    std::io::Read::by_ref(&mut file)
        .take(512 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= 512 * 1024 * 1024, "Hash input grew");
    let current = at(&parent, path.file_name().context("Hash name")?, NOFOLLOW, 0)?;
    ensure!(same(&file, &current)?, "Hash input replaced");
    sha(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::{PermissionsExt, symlink};
    #[test]
    fn descriptor_io_rejects_overwrite_symlink_and_replacement() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap()
            .join(".cache/automatic-mapping/native-export-conformance-2026-10-18");
        let path = root.join(format!("io-test-{}", uuid::Uuid::new_v4()));
        new_directory(&path).unwrap();
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o700
        );
        write(&path.join("private.json"), b"private sentinel").unwrap();
        assert_eq!(
            fs::metadata(path.join("private.json"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        assert!(write(&path.join("private.json"), b"replace").is_err());
        symlink("private.json", path.join("link.json")).unwrap();
        assert!(write(&path.join("link.json"), b"follow").is_err());
        let moved = path.with_extension("moved");
        fs::rename(&path, &moved).unwrap();
        fs::create_dir(&path).unwrap();
        assert!(write(&path.join("foreign.json"), b"leak").is_err());
        assert!(!path.join("foreign.json").exists());
        fs::remove_dir(&path).unwrap();
        fs::remove_dir_all(&moved).unwrap();
    }
}

/// Anchor recursive deletion to the held owned directory descriptor. Python's
/// standard fd-safe rmtree cannot traverse a replacement of the state pathname.
/// The final pathname removal is deliberately nonrecursive and requires empty.
fn remove_contents_fd(directory: &File) -> Result<()> {
    use std::os::unix::process::CommandExt;
    unsafe extern "C" {
        fn dup2(old: i32, new: i32) -> i32;
        fn fcntl(fd: i32, command: i32, ...) -> i32;
    }
    let source = directory.as_raw_fd();
    let mut command = Command::new("python3");
    command.args(["-I","-B","-c","import os,shutil,stat; assert shutil.rmtree.avoids_symlink_attacks\nfor name in os.listdir(7):\n mode=os.stat(name,dir_fd=7,follow_symlinks=False).st_mode\n if stat.S_ISDIR(mode): shutil.rmtree(name,dir_fd=7)\n else: os.unlink(name,dir_fd=7)\nassert os.listdir(7)==[]\n"])
        .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    // dup2/fcntl are async-signal-safe. Only the child's private descriptor table
    // changes; every parent descriptor keeps its original CLOEXEC protection.
    unsafe {
        command.pre_exec(move || {
            if dup2(source, 7) < 0 || fcntl(7, 2, 0) < 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    ensure!(
        command.status()?.success(),
        "Owned descriptor cleanup failed"
    );
    Ok(())
}
pub fn remove_owned_tree(path: &Path) -> Result<()> {
    let held = directory(path)?;
    ensure!(
        owners()
            .lock()
            .map_err(|_| anyhow::anyhow!("Ownership guard unavailable"))?
            .contains_key(path),
        "Deletion requires registered owned directory"
    );
    remove_contents_fd(&held)?;
    ensure!(
        same(&held, &directory(path)?)?,
        "State pathname replaced before empty-root removal"
    );
    ensure!(
        fs::read_dir(path)?.next().is_none(),
        "Owned state is not empty"
    );
    fs::remove_dir(path)?;
    Ok(())
}
#[cfg(test)]
mod cleanup_tests {
    use super::*;
    #[test]
    fn contents_cleanup_uses_held_directory_when_path_is_replaced() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap()
            .join(".cache/automatic-mapping/native-export-conformance-2026-10-18");
        let path = root.join(format!("cleanup-test-{}", uuid::Uuid::new_v4()));
        new_directory(&path).unwrap();
        write(&path.join("owned"), b"owned").unwrap();
        let held = directory(&path).unwrap();
        let moved = path.with_extension("moved");
        fs::rename(&path, &moved).unwrap();
        fs::create_dir(&path).unwrap();
        fs::write(path.join("foreign"), b"foreign sentinel").unwrap();
        remove_contents_fd(&held).unwrap();
        assert_eq!(fs::read(path.join("foreign")).unwrap(), b"foreign sentinel");
        assert!(fs::read_dir(&moved).unwrap().next().is_none());
        assert!(remove_owned_tree(&path).is_err());
        fs::remove_file(path.join("foreign")).unwrap();
        fs::remove_dir(path).unwrap();
        fs::remove_dir(moved).unwrap();
    }
}
