use crate::state::State;
use super::ptrace;
use anyhow::Result;

pub fn getcwd(state: &State, tid: ptrace::Pid, regs: ptrace::user_regs_struct, wait_ptrace_ret: impl Fn() -> Result<()>) -> Result<()> {
  let buf_ptr = ptrace::getreg!(regs, arg0);
  let buf_size = ptrace::getreg!(regs, arg1);
  let cwd = state.cwd.read().unwrap();
  ptrace::write_bytes(tid, buf_ptr, cwd.as_bytes(), buf_size as usize);
  ptrace::setregs(tid, ptrace::user_regs_struct {
    orig_rax: u64::MAX,
    ..regs
  }).unwrap();
  wait_ptrace_ret()?;
  ptrace::setregs(tid, ptrace::user_regs_struct {
    rax: 0,
    ..ptrace::getregs(tid).unwrap()
  }).unwrap();
  Ok(())
}