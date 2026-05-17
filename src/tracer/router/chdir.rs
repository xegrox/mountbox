use crate::state::State;
use super::{ptrace, Result};
use typed_path::NativePathBuf;

pub fn chdir(state: &State, tid: ptrace::Pid, regs: ptrace::user_regs_struct, wait_ptrace_ret: impl Fn() -> Result<()>) -> Result<()> {
  let relpath = NativePathBuf::from(ptrace::read_path(tid, ptrace::getreg!(regs, arg0))?);
  let path = state.cwd.read().unwrap().join(relpath);
  if let Some(_) = state.mounts.get_mount_of_path(&path) {
    *state.cwd.write().unwrap() = path; // TODO: emulate /proc/PID/cwd
    ptrace::setregs(tid, ptrace::user_regs_struct {
      orig_rax: u64::MAX,
      ..regs
    }).unwrap();
    wait_ptrace_ret()?;
    ptrace::setregs(tid, ptrace::user_regs_struct {
      rax: 0,
      ..ptrace::getregs(tid).unwrap()
    }).unwrap();
  } else {
    wait_ptrace_ret()?;
    if ptrace::getreg!(ptrace::getregs(tid)?, rax) == 0 {
      *state.cwd.write().unwrap() = path;
    }
  }
  Ok(())
}