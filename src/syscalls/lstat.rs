
use std::i64;
use std::mem::MaybeUninit;
use std::path::Path;
use std::rc::Rc;

use anyhow::Result;
use nix::errno::Errno;
use nix::libc;

use crate::fb::{req, res};
use crate::state::State;

pub fn serialize_req(state: &mut State, _mountpoint: Rc<Path>, path: &Path) -> Result<()> {
  state.fbb.reset();
  let fb_path = Some(state.fbb.create_string(&path.to_string_lossy()));
  let fb_stat = req::Stat::create(&mut state.fbb, &req::StatArgs {
    path: fb_path
  });
  let fb_req = req::Request::create(&mut state.fbb, &req::RequestArgs {
    operation_type: req::Operation::Stat,
    operation: Some(fb_stat.as_union_value())
  });
  state.fbb.finish(fb_req, None);
  Ok(())
}

pub fn deserialize_res(_state: &mut State, _mountpoint: Rc<Path>, _path: &Path, data: &Vec<u8>) -> Result<libc::stat, Errno> {
  if let Ok(response) = res::root_as_response(&data) {
    if let Some(fb_stat) = response.payload_as_stat() {
      let mut stat = unsafe { MaybeUninit::<libc::stat>::zeroed().assume_init() };
      stat.st_mode = match fb_stat.type_() {
          res::FileType::Directory => libc::S_IFDIR,
          res::FileType::File => libc::S_IFREG,
          _ => 0
      };
      stat.st_size = fb_stat.size_().try_into().unwrap_or_else(|_| i64::MAX);
      return Ok(stat);
    }
  }
  Err(Errno::EPERM)
}