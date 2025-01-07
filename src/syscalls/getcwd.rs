use nix::unistd::Pid;

use crate::{ptrace, state::State};

pub fn handler(state: &mut State, regs: &ptrace::user_regs_struct, pid: Pid) -> u64 {
  let size = ptrace::getreg!(regs, arg1) as usize;
  let addr = ptrace::getreg!(regs, arg0);
  let bytes = state.cwd.as_os_str().as_encoded_bytes();
  ptrace::write_bytes(pid, addr, &bytes[..std::cmp::min(size, bytes.len())]);
  return 0;
}