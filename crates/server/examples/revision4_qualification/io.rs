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
pub fn at(parent: &File, name: &std::ffi::OsStr, flags: i32, mode: u32) -> Result<File> {
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
pub fn same(a: &File, b: &File) -> Result<bool> {
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
    let mut file = at(
        &parent,
        path.file_name().context("Hash name")?,
        NOFOLLOW | NONBLOCK,
        0,
    )?;
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
    let current = at(
        &parent,
        path.file_name().context("Hash name")?,
        NOFOLLOW | NONBLOCK,
        0,
    )?;
    ensure!(same(&file, &current)?, "Hash input replaced");
    sha(&bytes)
}

pub fn read(path: &Path, maximum: usize) -> Result<Vec<u8>> {
    use std::io::Read;
    let parent = directory(path.parent().context("Input parent")?)?;
    let name = path.file_name().context("Input name")?;
    let mut f = at(&parent, name, NOFOLLOW | NONBLOCK, 0)?;
    let md = f.metadata()?;
    ensure!(
        md.is_file() && md.len() <= maximum as u64 && md.nlink() == 1,
        "Bounded regular input required"
    );
    let mut b = Vec::new();
    std::io::Read::by_ref(&mut f)
        .take(maximum as u64 + 1)
        .read_to_end(&mut b)?;
    ensure!(
        b.len() <= maximum && same(&f, &at(&parent, name, NOFOLLOW | NONBLOCK, 0)?)?,
        "Input changed or exceeded bound"
    );
    Ok(b)
}
pub fn exclusive(path: &Path) -> Result<File> {
    let parent = directory(path.parent().context("Private parent")?)?;
    let name = path.file_name().context("Private name")?;
    let f = at(&parent, name, 1 | CREATE_EXCL | NOFOLLOW, 0o600)?;
    let md = f.metadata()?;
    ensure!(
        md.is_file()
            && md.uid() == unsafe { geteuid() }
            && md.mode() & 0o777 == 0o600
            && md.nlink() == 1,
        "Private file identity invalid"
    );
    f.sync_all()?;
    parent.sync_all()?;
    Ok(f)
}
pub fn contained(root: &Path, text: &str) -> Result<std::path::PathBuf> {
    ensure!(
        !text.is_empty() && text.len() <= 4096 && !text.split('/').any(|x| x == "." || x == ".."),
        "Input path invalid"
    );
    let p = Path::new(text);
    let full = if p.is_absolute() {
        p.to_path_buf()
    } else {
        root.join(p)
    };
    ensure!(
        full.starts_with(root) && full != root,
        "Input outside repository"
    );
    directory(full.parent().context("Input parent")?)?;
    Ok(full)
}
pub fn strict(bytes: &[u8]) -> Result<Value> {
    use serde::de::{DeserializeSeed, Error, MapAccess, SeqAccess, Visitor};
    struct Seed<'a> {
        depth: usize,
        count: &'a std::cell::Cell<usize>,
    }
    impl<'de> DeserializeSeed<'de> for Seed<'_> {
        type Value = Value;
        fn deserialize<D: serde::Deserializer<'de>>(
            self,
            d: D,
        ) -> std::result::Result<Value, D::Error> {
            if self.depth > 32 || self.count.get() >= 1_000_000 {
                return Err(D::Error::custom("Bounded JSON"));
            }
            self.count.set(self.count.get() + 1);
            d.deserialize_any(self)
        }
    }
    impl<'de> Visitor<'de> for Seed<'_> {
        type Value = Value;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("bounded strict JSON")
        }
        fn visit_bool<E: Error>(self, v: bool) -> std::result::Result<Value, E> {
            Ok(Value::Bool(v))
        }
        fn visit_i64<E: Error>(self, v: i64) -> std::result::Result<Value, E> {
            Ok(v.into())
        }
        fn visit_u64<E: Error>(self, v: u64) -> std::result::Result<Value, E> {
            Ok(v.into())
        }
        fn visit_f64<E: Error>(self, v: f64) -> std::result::Result<Value, E> {
            serde_json::Number::from_f64(v)
                .map(Value::Number)
                .ok_or_else(|| E::custom("Finite JSON"))
        }
        fn visit_str<E: Error>(self, v: &str) -> std::result::Result<Value, E> {
            Ok(Value::String(v.into()))
        }
        fn visit_string<E: Error>(self, v: String) -> std::result::Result<Value, E> {
            Ok(Value::String(v))
        }
        fn visit_unit<E: Error>(self) -> std::result::Result<Value, E> {
            Ok(Value::Null)
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut a: A) -> std::result::Result<Value, A::Error> {
            let mut v = vec![];
            while let Some(x) = a.next_element_seed(Seed {
                depth: self.depth + 1,
                count: self.count,
            })? {
                v.push(x)
            }
            Ok(Value::Array(v))
        }
        fn visit_map<A: MapAccess<'de>>(self, mut a: A) -> std::result::Result<Value, A::Error> {
            let mut v = serde_json::Map::new();
            while let Some(k) = a.next_key::<String>()? {
                if v.contains_key(&k) {
                    return Err(A::Error::custom("Duplicate key"));
                }
                let x = a.next_value_seed(Seed {
                    depth: self.depth + 1,
                    count: self.count,
                })?;
                v.insert(k, x);
            }
            Ok(Value::Object(v))
        }
    }
    let count = std::cell::Cell::new(0);
    let mut d = serde_json::Deserializer::from_slice(bytes);
    let v = Seed {
        depth: 0,
        count: &count,
    }
    .deserialize(&mut d)?;
    d.end()?;
    Ok(v)
}
pub fn read_json(path: &Path, maximum: usize) -> Result<Value> {
    strict(&read(path, maximum)?)
}
pub fn fields(v: &Value, names: &[&str]) -> Result<()> {
    let o = v.as_object().context("Object required")?;
    ensure!(
        o.len() == names.len() && names.iter().all(|n| o.contains_key(*n)),
        "Exact fields required"
    );
    Ok(())
}
pub fn h(v: &Value) -> Result<&str> {
    let s = v.as_str().context("Hash string")?;
    ensure!(
        s.len() == 64
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "Hash encoding"
    );
    Ok(s)
}
pub fn value_hash(v: &Value) -> Result<String> {
    sha(&serde_json::to_vec(v)?)
}

#[cfg(target_os = "macos")]
const NONBLOCK: i32 = 0x4;
#[cfg(not(target_os = "macos"))]
const NONBLOCK: i32 = 0x800;
pub fn private_directory(path: &Path) -> Result<()> {
    let fd = directory(path)?;
    let m = fd.metadata()?;
    ensure!(
        m.uid() == unsafe { geteuid() } && m.mode() & 0o777 == 0o700,
        "Owned private directory required"
    );
    owners()
        .lock()
        .map_err(|_| anyhow::anyhow!("Ownership guard"))?
        .insert(path.into(), fd);
    Ok(())
}

pub fn remove_private_file(path: &Path) -> Result<Option<Value>> {
    use std::os::unix::ffi::OsStrExt;
    unsafe extern "C" {
        fn unlinkat(fd: i32, path: *const std::ffi::c_char, flags: i32) -> i32;
    }
    let parent = directory(path.parent().context("Redaction parent")?)?;
    let name = path.file_name().context("Redaction name")?;
    let file = match at(&parent, name, NOFOLLOW | NONBLOCK, 0) {
        Ok(f) => f,
        Err(e) => {
            if e.downcast_ref::<std::io::Error>()
                .is_some_and(|x| x.kind() == std::io::ErrorKind::NotFound)
            {
                return Ok(None);
            }
            return Err(e);
        }
    };
    let md = file.metadata()?;
    ensure!(
        md.is_file()
            && md.uid() == unsafe { geteuid() }
            && md.mode() & 0o777 == 0o600
            && md.nlink() == 1,
        "Exact private redaction target required"
    );
    let hash = hash_file(path)?;
    ensure!(
        same(&file, &at(&parent, name, NOFOLLOW | NONBLOCK, 0)?)?,
        "Redaction identity changed"
    );
    let c = CString::new(name.as_bytes())?;
    ensure!(
        unsafe { unlinkat(parent.as_raw_fd(), c.as_ptr(), 0) } == 0,
        "Owned credential redaction failed"
    );
    parent.sync_all()?;
    Ok(Some(
        serde_json::json!({"prior_sha256":hash,"bytes":md.len(),"values_printed":false,"removed":true}),
    ))
}
pub fn input_file(path: &Path, maximum: u64) -> Result<File> {
    let parent = directory(path.parent().context("Input parent")?)?;
    let file = at(
        &parent,
        path.file_name().context("Input name")?,
        NOFOLLOW | NONBLOCK,
        0,
    )?;
    let md = file.metadata()?;
    ensure!(
        md.is_file() && md.len() <= maximum && md.uid() == unsafe { geteuid() } && md.nlink() == 1,
        "Bounded private input required"
    );
    Ok(file)
}
