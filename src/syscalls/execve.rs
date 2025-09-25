use std::ffi::CString;
use std::fs::File;
use std::io::{Seek, SeekFrom, Write};
use std::os::fd::FromRawFd;
use std::{path::Path, rc::Rc};

use anyhow::{bail, Result};
use nix::unistd::Pid;
use crate::{ptrace, syscall_nr};
use crate::state::State;
use crate::syscalls::{read, open};


pub fn handler(state: &mut State, pid: Pid, mountpoint: Rc<Path>, path: &Path) -> Result<()> {
  let socket = state.mounts.get_mount_mut(&mountpoint).unwrap().socket.clone();
  let mut memfile = unsafe { File::from_raw_fd(state.execve_fd as i32) };
  let fd = {
    open::serialize_req(state, mountpoint.clone(), path)?;
    socket.borrow_mut().write(state.fbb.finished_data());
    let data = socket.borrow_mut().read();
    open::deserialize_res(state, mountpoint.clone(), path, &data)?.1
  } as u16;

  let mut read =  |mut buf: &mut [u8]| -> Result<u64> {
    read::serialize_req(state, mountpoint.clone(), fd, buf.len())?;
    socket.borrow_mut().write(state.fbb.finished_data());
    let data = socket.borrow_mut().read();
    let result = read::deserialize_res(state, mountpoint.clone(), fd, &data);
    match result {
      Ok((data, code)) => {
        buf.write(&data)?;
        Ok(code)
      }
      Err(err) => {
        bail!("Failed to read file: {}", err);
      }
    }
  };

  let mut buf = [0u8; 64*1024];
  let mut len = 0;
  while let buf_len = read(&mut buf)? && buf_len > 0 {
    len += buf_len;
    memfile.set_len(len)?;
    memfile.write_all(&buf[0..buf_len as usize])?;
    memfile.seek(SeekFrom::Current(buf_len as i64))?;
  }
  memfile.flush()?;

  let mut regs = ptrace::getregs(pid)?;
  ptrace::getreg!(regs, syscall_nr) = syscall_nr!(execveat);
  ptrace::getreg!(regs, arg0) = state.execve_fd as u64;
  let empty = CString::new("")?;
  ptrace::getreg!(regs, arg1) = empty.as_ptr() as u64;
  ptrace::getreg!(regs, arg2) = 0;
  ptrace::getreg!(regs, arg3) = 0;
  ptrace::getreg!(regs, arg4) = nix::libc::AT_EMPTY_PATH as u64;
  ptrace::setregs(pid, regs)?;
  ptrace::wait_syscall(pid)?;
  Ok(())
}