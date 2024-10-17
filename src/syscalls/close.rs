use std::path::Path;
use std::rc::Rc;

use anyhow::{anyhow, Result};
use nix::errno::Errno;

use crate::fb::{req, res};
use crate::state::State;

pub fn serialize_req(state: &mut State, _mountpoint: Rc<Path>, fd: u16) -> Result<()> {
  state.fbb.reset();
  let fd_desc = state.fd_allocator.get_desc_for_fd(fd).ok_or(anyhow!("Failed to find fd: {}", fd))?;
  let fb_fd_id = state.fbb.create_string(&fd_desc.id);
  let fb_fd = req::Fd::create(&mut state.fbb, &req::FdArgs {
    id: Some(fb_fd_id)
  });
  let fb_close = req::Close::create(&mut state.fbb, &req::CloseArgs {
    fd: Some(fb_fd)
  });
  let fb_req = req::Request::create(&mut state.fbb, &req::RequestArgs {
    operation_type: req::Operation::Close,
    operation: Some(fb_close.as_union_value())
  });
  state.fbb.finish(fb_req, None);
  Ok(())
}

pub fn deserialize_res(state: &mut State, _mountpoint: Rc<Path>, fd: u16, data: Vec<u8>) -> Result<(), Errno> {
  if let Ok(_) = res::root_as_response(&data) {
    state.fd_allocator.drop_fd(fd);
    return Ok(());
  }
  Err(Errno::EACCES)
}