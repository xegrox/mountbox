use std::path::Path;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use nix::errno::Errno;

use crate::fb::{req, res};
use crate::state::State;

pub fn serialize_req(state: &State, _mountpoint: Arc<Path>, fd: u16) -> Result<()> {
  let mut fbb = state.fbb.lock().unwrap();
  fbb.reset();
  let fd_alloc = state.fd_allocator.read().unwrap();
  let fd_desc = fd_alloc.get_desc_for_fd(fd).ok_or(anyhow!("Failed to find fd: {}", fd))?;
  let fb_fd_id = fbb.create_string(&fd_desc.id);
  let fb_fd = req::Fd::create(&mut fbb, &req::FdArgs {
    id: Some(fb_fd_id)
  });
  let fb_close = req::Close::create(&mut fbb, &req::CloseArgs {
    fd: Some(fb_fd)
  });
  let fb_req = req::Request::create(&mut fbb, &req::RequestArgs {
    operation_type: req::Operation::Close,
    operation: Some(fb_close.as_union_value())
  });
  fbb.finish(fb_req, None);
  Ok(())
}

pub fn deserialize_res(state: &State, _mountpoint: Arc<Path>, fd: u16, data: &Vec<u8>) -> Result<(), Errno> {
  if let Ok(_) = res::root_as_response(&data) {
    state.fd_allocator.write().unwrap().drop_fd(fd);
    return Ok(());
  }
  Err(Errno::EACCES)
}