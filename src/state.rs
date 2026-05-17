use std::{ffi::CString, os::fd::IntoRawFd, sync::RwLock};
use nix::sys::memfd::{memfd_create, MemFdCreateFlag};
use typed_path::NativePathBuf;

use crate::mounts::Mounts;

pub struct State {
  pub mounts: Mounts,
  pub cwd: RwLock<NativePathBuf>,
  pub execve_fd: RwLock<u16>
}

impl Default for State {
  fn default() -> Self {
    State {
      mounts: Mounts::new(&[]),
      cwd: RwLock::new(NativePathBuf::new()),
      execve_fd: RwLock::new(memfd_create(CString::new("mountbox").unwrap().as_c_str(), MemFdCreateFlag::empty()).unwrap().into_raw_fd() as u16)
    }
  }
}