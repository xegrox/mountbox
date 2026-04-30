use nix::{fcntl::readlink, libc::AT_FDCWD, unistd::Pid};
use typed_path::PlatformPathBuf;

pub fn resolve(pid: Pid, dirfd: i32, path: &str) -> PlatformPathBuf {
  if dirfd == AT_FDCWD {
    PlatformPathBuf::from(path)
  } else {
    let dirpath = PlatformPathBuf::from(readlink(format!("/proc/{}/fd/{}", pid.as_raw(), dirfd).as_str()).unwrap().as_encoded_bytes());
    dirpath.join(path)
  }
}