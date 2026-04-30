use anyhow::Result;
use nix::libc::user_regs_struct;
use typed_path::Utf8UnixPath;
use crate::mounts::Mount;
use super::ptrace;

pub fn open(mount: &Mount, path: &Utf8UnixPath, tid: ptrace::Pid, regs: user_regs_struct, wait_ptrace_ret: impl Fn() -> Result<()>) -> Result<()> {
  mount.plugin.open(path.as_str())?;
  let fd = mount.allocate_fd(path.as_str(), None)?;
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