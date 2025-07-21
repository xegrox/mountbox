use std::path::Path;
use std::rc::Rc;

use anyhow::Result;
use nix::errno::Errno;

use crate::fb::{req, res};
use crate::state::State;

pub fn serialize_req(state: &mut State, _mountpoint: Rc<Path>, path: &Path) -> Result<()> {
  state.fbb.reset();
  let fb_path = Some(state.fbb.create_string(&path.to_string_lossy()));
  let fb_open = req::Open::create(&mut state.fbb, &req::OpenArgs {
    path: fb_path
  });
  let fb_req = req::Request::create(&mut state.fbb, &req::RequestArgs {
    operation_type: req::Operation::Open,
    operation: Some(fb_open.as_union_value())
  });
  state.fbb.finish(fb_req, None);
  Ok(())
}

pub fn deserialize_res(state: &mut State, mountpoint: Rc<Path>, _path: &Path, data: &Vec<u8>) -> Result<((), u64), Errno> {
  if let Ok(response) = res::root_as_response(&data) {
    if let Some(fb_fd) = response.payload_as_fd() {
      return state.fd_allocator.allocate_fd(mountpoint, fb_fd.id()).map(|fd| ((), fd.into())).map_err(|_| Errno::EPERM);
    }
  }
  Err(Errno::EPERM)
}