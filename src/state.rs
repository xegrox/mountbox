use crate::fd_allocator::FdAllocator;

pub struct State<'a> {
  pub fd_allocator: FdAllocator<'a>
}