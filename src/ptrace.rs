use anyhow::{bail, Result};
pub use nix::{unistd::Pid, errno::Errno, libc::user_regs_struct};
use std::ffi::{c_long, c_void, CStr};
use nix::sys::{ptrace, wait::wait};

const LONG_LEN: usize = (c_long::BITS/8) as usize;

#[cfg(target_arch="x86_64")]
#[macro_export]
macro_rules! getreg {
  ($r:expr, syscall_nr) => { $r.orig_rax };
  ($r:expr, arg0) => { $r.rdi };
  ($r:expr, arg1) => { $r.rsi };
  ($r:expr, arg2) => { $r.rdx };
  ($r:expr, arg3) => { $r.r10 };
  ($r:expr, arg4) => { $r.r8 };
  ($r:expr, arg5) => { $r.r9 };
  ($r:expr, rip) => { $r.rip };
  ($r:expr, rax) => { $r.rax };
}

#[cfg(target_arch="x86_64")]
#[macro_export]
macro_rules! syscall_nr {
  (read) => { 0 };
  (open) => { 2 };
  (close) => { 3 };
  (stat) => { 4 };
  (fstat) => { 5 };
  (lstat) => { 6 };
  (getcwd) => { 79 };
  (openat) => { 257 };
  (statx) => { 332 };
}

pub use getreg;
pub use syscall_nr;

pub fn traceme() -> Result<(), Errno> {
  ptrace::traceme()
}

pub fn wait_syscall(pid: Pid) -> Result<(), Errno> {
  ptrace::syscall(pid, None)?;
  wait().map(drop)
}

pub fn getregs(pid: Pid) -> Result<user_regs_struct, Errno> {
  ptrace::getregs(pid)
}

pub fn read_str(pid: Pid, addr: u64) -> Result<String> {
  let mut data: Vec<u8> = Vec::new();
  while let Ok(chunk) = ptrace::read(pid, (addr as usize + data.len()) as *mut c_void) {
    let bytes = chunk.to_ne_bytes();
    data.extend(bytes);
    if bytes.contains(&0) {
      break;
    }
  }
  if data.is_empty() {
    bail!("ptrace: failed to read string")
  } else {
    Ok(CStr::from_bytes_until_nul(&data).unwrap().to_str().unwrap().to_string())
  }
}

pub fn write_bytes(pid: Pid, addr: u64, bytes: &[u8], len: usize) {
  let mut pos = 0;
  while pos < len {
    let chunk: [u8; LONG_LEN] = if pos+LONG_LEN > bytes.len() {
      let mut v = bytes[pos..bytes.len()].to_vec();
      v.resize(LONG_LEN, 0);
      v.try_into().unwrap()
    } else {
      bytes[pos..pos+LONG_LEN].try_into().unwrap()
    };
    ptrace::write(pid, (addr as usize + pos) as *mut c_void, c_long::from_ne_bytes(chunk)).unwrap();
    pos += LONG_LEN;
  }
}

pub fn fake_syscall(pid: Pid, regs: user_regs_struct, ret: u64) {
  ptrace::setregs(pid, user_regs_struct {
    orig_rax: u64::MAX,
    ..regs
  }).unwrap();
  ptrace::syscall(pid, None).unwrap();
  wait().unwrap();
  ptrace::setregs(pid, user_regs_struct {
    rax: ret,
    ..ptrace::getregs(pid).unwrap()
  }).unwrap();
}