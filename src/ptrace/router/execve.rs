use std::{ffi::CString, fs::File, io::Write, os::fd::FromRawFd, sync::RwLock};

use anyhow::Result;
use typed_path::Utf8UnixPath;
use crate::mounts::Mount;
use super::ptrace;

pub fn execve(mount: &Mount, path: &Utf8UnixPath, tid: ptrace::Pid, regs: ptrace::user_regs_struct, wait_ptrace_ret: impl Fn() -> Result<()>, execve_fd: &RwLock<u16>) -> Result<()> {
  mount.plugin.open(path.as_str())?;  
  let mut read_buf = [0u8; 64*1024];
  let mut len: u64 = 0;
  let memfile_fd = *execve_fd.read().unwrap();
  let mut memfile = unsafe { File::from_raw_fd(memfile_fd as i32) };
  while let Ok(read_len) = mount.plugin.read(path.as_str(), &mut read_buf, len as i64, 0) && read_len > 0 {
    len += read_len;
    memfile.set_len(len)?;
    memfile.write_all(&read_buf[0..read_len as usize])?;
  }
  memfile.flush()?;
  mount.plugin.close(path.as_str(), 0)?;
  
  let mut regs = regs.clone();
  ptrace::getreg!(regs, syscall_nr) = ptrace::syscall_nr!(execveat);
  ptrace::getreg!(regs, arg0) = memfile_fd as u64;
  let empty = CString::new("")?;
  ptrace::getreg!(regs, arg1) = empty.as_ptr() as u64;
  ptrace::getreg!(regs, arg2) = 0;
  ptrace::getreg!(regs, arg3) = 0;
  ptrace::getreg!(regs, arg4) = nix::libc::AT_EMPTY_PATH as u64;
  ptrace::setregs(tid, regs)?;
  wait_ptrace_ret()?;
  Ok(())
}