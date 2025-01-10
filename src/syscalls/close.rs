use flatbuffers::FlatBufferBuilder;
use nix::errno::Errno;

use crate::fb::{req, res};
use crate::state::State;

pub fn serialize_call<'a>(fbb: &'a mut FlatBufferBuilder, fd: u16, state: &mut State) -> &'a [u8] {
  fbb.reset();
  let fd_desc = state.fd_allocator.get_desc_for_fd(fd).unwrap();
  let fb_fd_id = fbb.create_string(&fd_desc.id);
  let fb_fd = req::Fd::create(fbb, &req::FdArgs {
    id: Some(fb_fd_id)
  });
  let fb_close = req::Close::create(fbb, &req::CloseArgs {
    fd: Some(fb_fd)
  });
  let fb_req = req::Request::create(fbb, &req::RequestArgs {
    operation_type: req::Operation::Close,
    operation: Some(fb_close.as_union_value())
  });
  fbb.finish(fb_req, None);
  return fbb.finished_data()
}

pub fn deserialize_ret(data: Vec<u8>, fd: u16, state: &mut State) -> Result<(), Errno> {
  if let Ok(_) = res::root_as_response(&data) {
    state.fd_allocator.drop_fd(fd);
    return Ok(());
  }
  Err(Errno::EACCES)
}