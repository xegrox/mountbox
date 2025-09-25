use std::path::PathBuf;

use nix::{fcntl::readlink, libc::AT_FDCWD, unistd::Pid};

pub struct DirFdResolver {

}

impl DirFdResolver {

  pub fn new() -> DirFdResolver {
    DirFdResolver {  }
  }

  pub fn resolve(&self, pid: Pid, dirfd: i32, path: &str) -> PathBuf {
    if dirfd == AT_FDCWD {
      PathBuf::from(path)
    } else {
      let dirpath = PathBuf::from(readlink(format!("/proc/{}/fd/{}", pid.as_raw(), dirfd).as_str()).unwrap());
      dirpath.join(path)
    }
  }
}