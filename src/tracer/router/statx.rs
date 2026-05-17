use std::mem::MaybeUninit;
use nix::libc::user_regs_struct;
use typed_path::Utf8UnixPath;
use crate::{mounts::Mount, plugin};
use super::{ptrace, Result};

pub fn statx(mount: &Mount, path: &Utf8UnixPath, tid: ptrace::Pid, regs: user_regs_struct, wait_ptrace_ret: impl Fn() -> Result<()>) -> Result<()> {
  let stat = mount.plugin.getattr(path.as_str())?;
  let mut cstatx = unsafe { MaybeUninit::<nix::libc::statx>::zeroed().assume_init() };
  match stat.mode & plugin::S_IFMT {
    plugin::S_IFREG => cstatx.stx_mode |= nix::libc::S_IFREG as u16,
    plugin::S_IFDIR => cstatx.stx_mode |= nix::libc::S_IFDIR as u16,
    plugin::S_IFLNK => cstatx.stx_mode |= nix::libc::S_IFLNK as u16,
    _ => {}
  }
  cstatx.stx_mode |= 0o777;
  cstatx.stx_size = stat.size;
  cstatx.stx_atime.tv_sec = stat.atime;
  cstatx.stx_mtime.tv_sec = stat.mtime;
  cstatx.stx_ctime.tv_sec = stat.ctime;
  let cstatx_buf = unsafe { core::slice::from_raw_parts(
    (&cstatx as *const nix::libc::statx) as *const u8,
    core::mem::size_of::<nix::libc::statx>(),
  ) };
  let buf_ptr = ptrace::getreg!(regs, arg2);
  ptrace::write_bytes(tid, buf_ptr, cstatx_buf, cstatx_buf.len())?;
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