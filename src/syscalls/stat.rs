
use std::i64;
use std::mem::MaybeUninit;
use std::path::Path;

use flatbuffers::FlatBufferBuilder;
use nix::errno::Errno;
use nix::libc;

use crate::fb::{req, res};
use crate::state::State;

pub fn serialize_call<'a>(path: &Path, fbb: &'a mut FlatBufferBuilder, _: &mut State) -> &'a [u8] {
  fbb.reset();
  let fb_path = Some(fbb.create_string(path.to_str().unwrap()));
  let fb_stat = req::Stat::create(fbb, &req::StatArgs {
    path: fb_path
  });
  let fb_req = req::Request::create(fbb, &req::RequestArgs {
    operation_type: req::Operation::Stat,
    operation: Some(fb_stat.as_union_value())
  });
  fbb.finish(fb_req, None);
  return fbb.finished_data()
}

pub fn deserialize_ret(_: &Path, data: Vec<u8>, _: &mut State) -> Result<libc::stat, Errno> {
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