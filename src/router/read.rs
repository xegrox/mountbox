use anyhow::Result;
use nix::libc::user_regs_struct;
use typed_path::Utf8UnixPath;
use crate::{mounts::Mount, ptrace};

pub fn read(mount: &Mount, path: &Utf8UnixPath, tid: ptrace::Pid, regs: user_regs_struct, wait_ptrace_ret: impl Fn() -> Result<()>) -> Result<()> {
  let buf_ptr = ptrace::getreg!(regs, arg1);
  let buf_size = ptrace::getreg!(regs, arg2);
  let data = mount.plugin.read(path.as_str(), buf_size, 0)?;
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