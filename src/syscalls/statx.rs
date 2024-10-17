
use std::mem::MaybeUninit;

use flatbuffers::FlatBufferBuilder;
use nix::libc::{self, statx};
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
    if let Some(stat) = response.payload_as_stat() {
      let mut statx = unsafe { MaybeUninit::<statx>::zeroed().assume_init() };
      statx.stx_mode = match stat.type_() {
          res::FileType::Directory => libc::S_IFDIR as u16,
          res::FileType::File => libc::S_IFREG as u16,
          _ => 0
      };
      statx.stx_size = stat.size_();
      ptrace::write(pid, ptrace::getreg!(regs, arg4), &statx);
      return 0;
    }
  }
  return 1u64.wrapping_neg();
}