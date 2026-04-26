use std::{ffi::CString, os::fd::IntoRawFd, path::PathBuf, sync::RwLock};
use nix::sys::memfd::{memfd_create, MemFdCreateFlag};

use crate::{dirfd_resolver::DirFdResolver, mounts::Mounts};

pub struct State {
  pub mounts: Mounts,
  pub cwd: RwLock<PathBuf>,
  pub dirfd_resolver: DirFdResolver,
  pub execve_fd: RwLock<u16>
}

impl Default for State {
  fn default() -> Self {
    State {
      mounts: Mounts::new(&[]),
      cwd: RwLock::new(PathBuf::new()),
      dirfd_resolver: DirFdResolver::new(),
      execve_fd: RwLock::new(memfd_create(CString::new("name").unwrap().as_c_str(), MemFdCreateFlag::empty()).unwrap().into_raw_fd() as u16)
    }
  }
}