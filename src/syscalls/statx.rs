
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

pub fn deserialize_res(_state: &mut State, _mountpoint: Rc<Path>, _path: &Path, data: Vec<u8>) -> Result<libc::statx, Errno> {
  if let Ok(response) = res::root_as_response(&data) {
    if let Some(stat) = response.payload_as_stat() {
      let mut statx = unsafe { MaybeUninit::<libc::statx>::zeroed().assume_init() };
      statx.stx_mode = match stat.type_() {
          res::FileType::Directory => libc::S_IFDIR as u16,
          res::FileType::File => libc::S_IFREG as u16,
          _ => 0
      };
      statx.stx_size = stat.size_();
      return Ok(statx);
    }
  }
  Err(Errno::EPERM)
}