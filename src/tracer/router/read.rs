use nix::libc::user_regs_struct;
use crate::mounts::Mount;
use super::{ptrace, Result};

pub fn read(mount: &Mount, fd: u16, tid: ptrace::Pid, regs: user_regs_struct, wait_ptrace_ret: impl Fn() -> Result<()>) -> Result<()> {
  let fd_info = mount.get_fd_info(fd).unwrap();
  let buf_ptr = ptrace::getreg!(regs, arg1);
  let buf_size = ptrace::getreg!(regs, arg2);
  // TODO: impl offset
  let mut read_buf = vec![0u8; buf_size as usize];
  let read_len = mount.plugin.read(fd_info.path.as_str(), &mut read_buf, 0, fd_info.fh)?;
  ptrace::write_bytes(tid, buf_ptr, &read_buf, read_len as usize)?;
  ptrace::setregs(tid, user_regs_struct {
    orig_rax: u64::MAX,
    ..regs
  }).unwrap();
  wait_ptrace_ret()?;
  ptrace::setregs(tid, user_regs_struct {
    rax: read_len as u64,
    ..ptrace::getregs(tid).unwrap()
  }).unwrap();
  Ok(())
}