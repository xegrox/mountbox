use std::path::Path;
use std::rc::Rc;

use anyhow::{anyhow, Result};
use nix::errno::Errno;

use crate::fb::req;
use crate::state::State;

pub fn serialize_req(state: &mut State, _mountpoint: Rc<Path>, fd: u16, len: usize) -> Result<()> {
  state.fbb.reset();
  let fd_desc = state.fd_allocator.get_desc_for_fd(fd).ok_or(anyhow!("Failed to find fd: {}", fd))?;
  let fb_fd_id = state.fbb.create_string(&fd_desc.id);
  let fb_fd = req::Fd::create(&mut state.fbb, &req::FdArgs {
    id: Some(fb_fd_id)
  });
  let fb_read = req::Read::create(&mut state.fbb, &req::ReadArgs {
    fd: Some(fb_fd),
    len: len.try_into()?
  });
  let fb_req = req::Request::create(&mut state.fbb, &req::RequestArgs {
    operation_type: req::Operation::Read,
    operation: Some(fb_read.as_union_value())
  });
  state.fbb.finish(fb_req, None);
  Ok(())
}

pub fn deserialize_res(_state: &mut State, _mountpoint: Rc<Path>, _fd: u16, data: Vec<u8>) -> Result<Vec<u8>, Errno> {
  Ok(data)
}