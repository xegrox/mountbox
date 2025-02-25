use std::path::PathBuf;

use nix::libc::AT_FDCWD;

pub struct DirFdResolver {

}

impl DirFdResolver {

  pub fn new() -> DirFdResolver {
    DirFdResolver {  }
  }

  pub fn resolve(&self, dirfd: i32, path: &str) -> PathBuf {
    if dirfd == AT_FDCWD {
      PathBuf::from(path)
    } else {
      unimplemented!()
    }
  }
}