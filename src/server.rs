use anyhow::{bail, Result};
use std::{cmp::Ordering, collections::HashMap, path::Path};
use nix::{sys::wait::wait, unistd::Pid};

use crate::{ptrace, sockets::Socket, state::State, syscalls};

pub fn run<'a>(pid: Pid, mut mountsockets: HashMap<&'a Path, Box<dyn Socket>>, state: &mut State<'a>) -> Result<()> {
  let mut fbb = flatbuffers::FlatBufferBuilder::new();
  let mut mountpoints = mountsockets.iter().map(|(&p, _)| p).collect::<Vec<&Path>>();
  mountpoints.sort_unstable_by(|a, b| { a.cmp(b) });
  if mountpoints.len() > 0 {
    for i in 0..mountsockets.len() - 1 {
      if mountpoints[i].cmp(&mountpoints[i+1]) == Ordering::Equal {
        bail!("Mount path {} specified more than once", mountpoints[i].display())
      }
    }
  }

  let get_parent_mountpoint = |path: &Path| -> Option<&Path> {
    for mountp in mountpoints.iter() {
      if path.starts_with(mountp) {
        return Some(mountp);
      }
    }
    None
  };

  macro_rules! handler {
    ($pid:expr, $regs:expr, $s:tt) => {{
      let ret = syscalls::$s::handler(state, &$regs, $pid);
      ptrace::fake_syscall($pid, $regs, ret);
    }};
  }

  macro_rules! handler_path {
    ($pid:expr, $regs:expr, $s:tt, $arg:tt) => {{
      if let Ok(path) = ptrace::read_str($pid, ptrace::getreg!($regs, $arg)) {
        let fullpath = state.cwd.join(path);
        let mountp = get_parent_mountpoint(fullpath.as_path());
        if let Some(mountp) = mountp {
          fbb.reset();
          let relpath = Path::new("/").join(fullpath.strip_prefix(mountp).unwrap());
          let data = syscalls::$s::serialize_call(&relpath, &mut fbb, state, &$regs, $pid);
          let socket = mountsockets.get_mut(&mountp).unwrap();
          socket.write(data);
          state.fd_allocator.set_current_mountpoint(mountp);
          let ret = syscalls::$s::deserialize_ret(&relpath, socket.read(), state, &$regs, $pid);
          ptrace::fake_syscall($pid, $regs, ret);
        } else {
          ptrace::wait_syscall($pid).unwrap();
        }
      }
    }};
  }

  macro_rules! handler_fd {
    ($pid:expr, $regs:expr, $syscall:tt, $arg:tt) => {{
      let fd = ptrace::getreg!($regs, $arg).try_into().unwrap();
      if let Some(fd_desc) = state.fd_allocator.get_desc_for_fd(fd) {
        let mountpoint = fd_desc.mountpoint.to_path_buf();
        fbb.reset();
        let data = syscalls::$syscall::serialize_call(&mut fbb, fd, state, &$regs, $pid);
        let socket = mountsockets.get_mut(mountpoint.as_path()).unwrap();
        socket.write(data);
        let ret = syscalls::$syscall::deserialize_ret(socket.read(), fd, state, &$regs, $pid);
        ptrace::fake_syscall($pid, $regs, ret);
      } else {
        ptrace::wait_syscall($pid).unwrap();
      }
    }};
  }

  wait().unwrap();
  while let Ok(_) = ptrace::wait_syscall(pid) {
    let regs = ptrace::getregs(pid).unwrap();
    match ptrace::getreg!(regs, syscall_nr) {
      ptrace::syscall_nr!(open) => handler_path!(pid, regs, open, arg0),
      ptrace::syscall_nr!(close) => handler_fd!(pid, regs, close, arg0),
      ptrace::syscall_nr!(stat) => handler_path!(pid, regs, stat, arg0),
      ptrace::syscall_nr!(fstat) => handler_fd!(pid, regs, fstat, arg0),
      ptrace::syscall_nr!(lstat) => handler_path!(pid, regs, lstat, arg0),
      ptrace::syscall_nr!(statx) => handler_path!(pid, regs, statx, arg1),
      ptrace::syscall_nr!(getcwd) => handler!(pid, regs, getcwd),
      _ => {
        ptrace::wait_syscall(pid).unwrap();
      }
    }
  };
  Ok(())
}