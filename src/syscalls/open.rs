use std::path::Path;

use flatbuffers::FlatBufferBuilder;
use nix::errno::Errno;

use crate::fb::{req, res};
use crate::state::State;

pub fn serialize_call<'a>(path: &Path, fbb: &'a mut FlatBufferBuilder, _: &mut State) -> &'a [u8] {
  fbb.reset();
  let fb_path = Some(fbb.create_string(path.to_str().unwrap()));
  let fb_open = req::Open::create(fbb, &req::OpenArgs {
    path: fb_path
  });
  let fb_req = req::Request::create(fbb, &req::RequestArgs {
    operation_type: req::Operation::Open,
    operation: Some(fb_open.as_union_value())
  });
  fbb.finish(fb_req, None);
  return fbb.finished_data()
}

pub fn deserialize_ret(_: &Path, data: Vec<u8>, state: &mut State) -> Result<u64, Errno> {
  if let Ok(response) = res::root_as_response(&data) {
    if let Some(fb_fd) = response.payload_as_fd() {
      return Ok(state.fd_allocator.allocate_fd(fb_fd.id().unwrap()).unwrap().into());
    }
  }
  Err(Errno::EPERM)
}