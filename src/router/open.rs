use std::path::Path;
use anyhow::Result;
use nix::libc::user_regs_struct;
use crate::{mounts::Mount, ptrace,};

pub fn open(mount: &Mount, path: &Path, tid: ptrace::Pid, regs: user_regs_struct, wait_ptrace_ret: impl Fn() -> Result<()>) -> Result<()> {
  mount.plugin.open(path.to_str().unwrap())?;
  let fd = mount.allocate_fd(path.to_path_buf(), None)?;
  ptrace::setregs(tid, user_regs_struct {
    orig_rax: u64::MAX,
    ..regs
  }).unwrap();
  wait_ptrace_ret()?;
  ptrace::setregs(tid, user_regs_struct {
    rax: fd.into(),
    ..ptrace::getregs(tid).unwrap()
  }).unwrap();
  Ok(())
}