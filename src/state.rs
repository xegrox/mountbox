use std::path::PathBuf;

use crate::fd_allocator::FdAllocator;

pub struct State<'a> {
  pub fd_allocator: FdAllocator<'a>,
  pub cwd: PathBuf
}

impl<'a> Default for State<'a> {
  fn default() -> Self {
      State { fd_allocator: FdAllocator::new(), cwd: PathBuf::new() }
  }
}