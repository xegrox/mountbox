
use std::i64;
use std::mem::MaybeUninit;

use flatbuffers::FlatBufferBuilder;
use nix::libc;
use nix::unistd::Pid;

use crate::fb::{req, res};
use crate::ptrace;
use crate::state::State;

pub fn serialize_call<'a>(path: &str, fbb: &'a mut FlatBufferBuilder, _: &mut State, _: &ptrace::user_regs_struct, _: Pid) -> &'a [u8] {
  fbb.reset();
  let fb_path = Some(fbb.create_string(path));
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

pub fn deserialize_ret(_: &str, data: Vec<u8>, _: &mut State, regs: &ptrace::user_regs_struct, pid: Pid) -> u64 {
  if let Ok(response) = res::root_as_response(&data) {
    if let Some(fb_stat) = response.payload_as_stat() {
      let mut stat = unsafe { MaybeUninit::<libc::stat>::zeroed().assume_init() };
      stat.st_mode = match fb_stat.type_() {
          res::FileType::Directory => libc::S_IFDIR,
          res::FileType::File => libc::S_IFREG,
          _ => 0
      };
      stat.st_size = fb_stat.size_().try_into().unwrap_or_else(|_| i64::MAX);
      ptrace::write(pid, ptrace::getreg!(regs, arg1), &stat);
      return 0;
    }
  }
  return 1u64.wrapping_neg();
}