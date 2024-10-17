use std::{collections::HashMap, path::PathBuf};

use flatbuffers::FlatBufferBuilder;

use crate::{dirfd_resolver::DirFdResolver, fd_allocator::FdAllocator, mounts::Mounts};

pub struct State {
  pub mounts: Mounts,
  pub fd_allocator: FdAllocator,
  pub fbb: FlatBufferBuilder<'static>,
  pub cwd: PathBuf,
  pub dirfd_resolver: DirFdResolver
}

impl Default for State {
  fn default() -> Self {
    State {
      mounts: Mounts::new(HashMap::new()),
      fd_allocator: FdAllocator::new(),
      cwd: PathBuf::new(),
      fbb: FlatBufferBuilder::new(),
      dirfd_resolver: DirFdResolver::new()
    }
  }
}