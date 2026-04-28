use std::mem::MaybeUninit;
use anyhow::Result;
use nix::libc::user_regs_struct;
use crate::{mounts::Mount, ptrace};

pub fn fstat(mount: &Mount, fd: u16, tid: ptrace::Pid, regs: user_regs_struct, wait_ptrace_ret: impl Fn() -> Result<()>) -> Result<()> {
  let fd_info = mount.get_fd_info(fd).unwrap();
  let stat = mount.plugin.getattr(fd_info.path.as_str())?;
  let mut cstat = unsafe { MaybeUninit::<nix::libc::stat>::zeroed().assume_init() };
  cstat.st_size = stat.size as nix::libc::off_t;
  cstat.st_atime = stat.atime;
  cstat.st_mtime = stat.mtime;
  cstat.st_ctime = stat.ctime;
  let cstat_buf = unsafe { core::slice::from_raw_parts(
    (&cstat as *const nix::libc::stat) as *const u8,
    core::mem::size_of::<nix::libc::stat>(),
  ) };
  let buf_ptr = ptrace::getreg!(regs, arg1);
  ptrace::write_bytes(tid, buf_ptr, cstat_buf, cstat_buf.len());
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