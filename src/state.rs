use std::{collections::HashMap, ffi::CString, os::fd::IntoRawFd, path::PathBuf};

use flatbuffers::FlatBufferBuilder;
use nix::sys::memfd::{memfd_create, MemFdCreateFlag};

use crate::{dirfd_resolver::DirFdResolver, fd_allocator::FdAllocator, mounts::Mounts};

pub struct State {
  pub mounts: Mounts,
  pub fd_allocator: FdAllocator,
  pub fbb: FlatBufferBuilder<'static>,
  pub cwd: PathBuf,
  pub dirfd_resolver: DirFdResolver,
  pub execve_fd: u16
}

impl Default for State {
  fn default() -> Self {
    State {
      mounts: Mounts::new(HashMap::new()),
      fd_allocator: FdAllocator::new(),
      cwd: PathBuf::new(),
      fbb: FlatBufferBuilder::new(),
      dirfd_resolver: DirFdResolver::new(),
      execve_fd: memfd_create(CString::new("name").unwrap().as_c_str(), MemFdCreateFlag::empty()).unwrap().into_raw_fd() as u16
    }
  }
}