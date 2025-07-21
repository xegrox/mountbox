use std::path::Path;
use std::rc::Rc;

use anyhow::{anyhow, Result};
use nix::errno::Errno;

use crate::fb::req;
use crate::fb::res;
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

pub fn deserialize_res<'a>(_state: &mut State, _mountpoint: Rc<Path>, _fd: u16, data: &'a Vec<u8>) -> Result<(&'a [u8], u64), Errno> {
  if let Ok(response) = res::root_as_response(&data) {
    if let Some(fb_read) = response.payload_as_read() {
      let data = fb_read.data().bytes();
      let len = data.len();
      return Ok((data, len.try_into().unwrap()));
    }
  }
  Err(Errno::EPERM)
}