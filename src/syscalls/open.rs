use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use nix::errno::Errno;

use crate::fb::{req, res};
use crate::state::State;

pub fn serialize_req(state: &State, _mountpoint: Arc<Path>, path: &Path) -> Result<()> {
  let mut fbb = state.fbb.lock().unwrap();
  fbb.reset();
  let fb_path = Some(fbb.create_string(&path.to_string_lossy()));
  let fb_open = req::Open::create(&mut fbb, &req::OpenArgs {
    path: fb_path
  });
  let fb_req = req::Request::create(&mut fbb, &req::RequestArgs {
    operation_type: req::Operation::Open,
    operation: Some(fb_open.as_union_value())
  });
  fbb.finish(fb_req, None);
  Ok(())
}

pub fn deserialize_res(state: &State, mountpoint: Arc<Path>, _path: &Path, data: &Vec<u8>) -> Result<((), u64), Errno> {
  if let Ok(response) = res::root_as_response(&data) {
    if let Some(fb_fd) = response.payload_as_fd() {
      return state.fd_allocator.write().unwrap().allocate_fd(mountpoint, fb_fd.id()).map(|fd| ((), fd.into())).map_err(|_| Errno::EPERM);
    }
  }
  Err(Errno::EPERM)
}