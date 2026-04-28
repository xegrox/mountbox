use anyhow::Result;
use nix::libc::user_regs_struct;
use crate::{mounts::Mount, ptrace,};

pub fn close(mount: &Mount, fd: u16, tid: ptrace::Pid, regs: user_regs_struct, wait_ptrace_ret: impl Fn() -> Result<()>) -> Result<()> {
  let fd_info = mount.get_fd_info(fd).unwrap();
  mount.plugin.close(fd_info.path.as_str(), fd_info.fh)?;
  drop(fd_info);
  mount.release_fd(fd);
  ptrace::setregs(tid, user_regs_struct {
    orig_rax: u64::MAX,
    ..regs
  }).unwrap();
  wait_ptrace_ret()?;
  ptrace::setregs(tid, user_regs_struct {
    rax: 0,
    ..ptrace::getregs(tid).unwrap()
  }).unwrap();
  Ok(())
}