
use std::i64;
use std::mem::MaybeUninit;

use nix::errno::Errno;
use nix::libc;

use crate::fb::{req, res};
use crate::state::State;

pub fn serialize_call<'a>(fbb: &'a mut flatbuffers::FlatBufferBuilder, fd: u16, state: &mut State) -> &'a [u8] {
  fbb.reset();
  let fd_desc = state.fd_allocator.get_desc_for_fd(fd).unwrap();
  let fb_fd_id = fbb.create_string(&fd_desc.id);
  let fb_fd = req::Fd::create(fbb, &req::FdArgs {
    id: Some(fb_fd_id)
  });
  let fb_fstat = req::Fstat::create(fbb, &req::FstatArgs {
    fd: Some(fb_fd)
  });
  let fb_req = req::Request::create(fbb, &req::RequestArgs {
    operation_type: req::Operation::Fstat,
    operation: Some(fb_fstat.as_union_value())
  });
  fbb.finish(fb_req, None);
  return fbb.finished_data()
}

pub fn deserialize_ret(data: Vec<u8>, _: u16, _: &mut State) -> Result<libc::stat, Errno> {
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
  Err(Errno::EACCES)
}