use nix::{fcntl::readlink, libc::AT_FDCWD, unistd::Pid};
use typed_path::NativePathBuf;

pub fn resolve(pid: Pid, dirfd: i32, path: &str) -> NativePathBuf {
  if dirfd == AT_FDCWD {
    NativePathBuf::from(path)
  } else {
    let dirpath = NativePathBuf::from(readlink(format!("/proc/{}/fd/{}", pid.as_raw(), dirfd).as_str()).unwrap().as_encoded_bytes());
    dirpath.join(path)
  }
}