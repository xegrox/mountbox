
use std::i64;
use std::mem::MaybeUninit;
use std::path::Path;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use nix::errno::Errno;
use nix::libc;

use crate::fb::{req, res};
use crate::state::State;

pub fn serialize_req(state: &State, _mountpoint: Arc<Path>, fd: u16) -> Result<()> {
  let mut fbb = state.fbb.lock().unwrap();
  fbb.reset();
  let fd_alloc = state.fd_allocator.read().unwrap();
  let fd_desc = fd_alloc.get_desc_for_fd(fd).ok_or(anyhow!("Failed to find fd: {}", fd))?;
  let fb_fd_id = fbb.create_string(&fd_desc.id);
  let fb_fd = req::Fd::create(&mut fbb, &req::FdArgs {
    id: Some(fb_fd_id)
  });
  let fb_fstat = req::Fstat::create(&mut fbb, &req::FstatArgs {
    fd: Some(fb_fd)
  });
  let fb_req = req::Request::create(&mut fbb, &req::RequestArgs {
    operation_type: req::Operation::Fstat,
    operation: Some(fb_fstat.as_union_value())
  });
  fbb.finish(fb_req, None);
  Ok(())
}

pub fn deserialize_res(_state: &State, _mountpoint: Arc<Path>, _fd: u16, data: &Vec<u8>) -> Result<libc::stat, Errno> {
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