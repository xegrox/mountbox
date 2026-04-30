use anyhow::Result;
use nix::libc::user_regs_struct;
use crate::mounts::Mount;
use super::ptrace;

pub fn read(mount: &Mount, fd: u16, tid: ptrace::Pid, regs: user_regs_struct, wait_ptrace_ret: impl Fn() -> Result<()>) -> Result<()> {
  let fd_info = mount.get_fd_info(fd).unwrap();
  let buf_ptr = ptrace::getreg!(regs, arg1);
  let buf_size = ptrace::getreg!(regs, arg2);
  // TODO: impl offset
  let data = mount.plugin.read(fd_info.path.as_str(), buf_size, 0, fd_info.fh)?;
  ptrace::write_bytes(tid, buf_ptr, &data, data.len());
  ptrace::setregs(tid, user_regs_struct {
    orig_rax: u64::MAX,
    ..regs
  }).unwrap();
  wait_ptrace_ret()?;
  ptrace::setregs(tid, user_regs_struct {
    rax: data.len() as u64,
    ..ptrace::getregs(tid).unwrap()
  }).unwrap();
  Ok(())
}