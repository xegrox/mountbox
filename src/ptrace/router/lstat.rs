use std::mem::MaybeUninit;
use anyhow::Result;
use nix::libc::user_regs_struct;
use typed_path::Utf8UnixPath;
use crate::{mounts::Mount, plugin};
use super::ptrace;

pub fn lstat(mount: &Mount, path: &Utf8UnixPath, tid: ptrace::Pid, regs: user_regs_struct, wait_ptrace_ret: impl Fn() -> Result<()>) -> Result<()> {
  let stat = mount.plugin.getattr(path.as_str())?;
  let mut cstat = unsafe { MaybeUninit::<nix::libc::stat>::zeroed().assume_init() };
  match stat.mode & plugin::S_IFMT {
    plugin::S_IFREG => cstat.st_mode |= nix::libc::S_IFREG,
    plugin::S_IFDIR => cstat.st_mode |= nix::libc::S_IFDIR,
    plugin::S_IFLNK => cstat.st_mode |= nix::libc::S_IFLNK,
    _ => {}
  }
  cstat.st_mode |= 0o777;
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