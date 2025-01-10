use anyhow::{bail, Result};
use std::{cmp::Ordering, collections::HashMap, ffi::OsStr, path::Path};
use nix::{sys::wait::wait, unistd::Pid};

use crate::{ptrace, sockets::Socket, state::State, syscalls};

trait ToBytes {
  fn to_bytes(&self) -> &[u8];
}

impl ToBytes for OsStr {
  fn to_bytes(&self) -> &[u8] {
    self.as_encoded_bytes()
  }
}

impl<T: Sized> ToBytes for T {
  fn to_bytes(&self) -> &[u8] {
    unsafe {
      ::core::slice::from_raw_parts(
        (self as *const T) as *const u8,
        ::core::mem::size_of::<T>(),
      )
    }
  }
}

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

  // macro_rules! parse_args {
  //   ($pid: expr, $regs:expr, [$i:expr]str) => {(||{
      
  //   })()};
  //   ($pid: expr, $regs:expr, [$i:expr]u64) => {(||{
      
  //   })()};
  //   ($pid: expr, $regs:expr, [$i:expr]$t:tt, $($others:tt)*) => {
      
  //     (parse_args!([$i]$t), parse_args!($($others)*))
  //   }
  // }

  macro_rules! route_all {
    ($pid:expr, $regs:expr, $mod:tt, |$ret:ident| $get_code:block) => {{
      let code = match syscalls::$mod::handler(state) {
        Ok($ret) => $get_code,
        Err(err) => err as u64
      };
      ptrace::fake_syscall($pid, $regs, code);
    }};
    ($pid:expr, $regs:expr, $mod:tt, ret_code) => {
      route_all!($pid, $regs, $mod, |_ret| {
        _ret
      })
    };
    ($pid:expr, $regs:expr, $mod:tt $(, ret_data $ret_arg:tt $([$ret_len_arg:tt])?)?) => {
      route_all!($pid, $regs, $mod, |_ret| {
        $(
          let bytes = _ret.to_bytes();
          let _len = bytes.len();
          $(let _len = ptrace::getreg!($regs, $ret_len_arg) as usize;)?
          ptrace::write_bytes(pid, ptrace::getreg!($regs, $ret_arg), bytes, _len);
        )?
        0
      })
    };
  }

  macro_rules! route_path {
    ($pid:expr, $regs:expr, $mod:tt, $path_arg:tt, |$ret:ident| $get_code:block) => {{
      if let Ok(path) = ptrace::read_str($pid, ptrace::getreg!($regs, $path_arg)) {
        let fullpath = state.cwd.join(path);
        let mountp = get_parent_mountpoint(fullpath.as_path());
        if let Some(mountp) = mountp {
          fbb.reset();
          let relpath = Path::new("/").join(fullpath.strip_prefix(mountp).unwrap());
          let data = syscalls::$mod::serialize_call(&relpath, &mut fbb, state);
          let socket = mountsockets.get_mut(&mountp).unwrap();
          socket.write(data);
          state.fd_allocator.set_current_mountpoint(mountp);
          let code = match syscalls::$mod::deserialize_ret(&relpath, socket.read(), state) {
            Ok($ret) => $get_code,
            Err(err) => err as u64
          };
          ptrace::fake_syscall($pid, $regs, code);
        } else {
          ptrace::wait_syscall($pid).unwrap();
        }
      }
    }};
    ($pid:expr, $regs:expr, $mod:tt, $path_arg:tt, ret_code) => {
      route_path!($pid, $regs, $mod, $path_arg, |_ret| {
        _ret
      })
    };
    ($pid:expr, $regs:expr, $mod:tt, $path_arg:tt $(, ret_data $ret_arg:tt $([$ret_len_arg:tt])?)?) => {
      route_path!($pid, $regs, $mod, $path_arg, |_ret| {
        $(
          let bytes = _ret.to_bytes();
          let _len = bytes.len();
          $(let _len = ptrace::getreg!($regs, $ret_len_arg) as usize;)?
          ptrace::write_bytes(pid, ptrace::getreg!($regs, $ret_arg), bytes, _len);
        )?
        0
      })
    };
  }

  macro_rules! route_fd {
    ($pid:expr, $regs:expr, $syscall:tt, $fd_arg:tt, |$ret:ident| $get_code:block) => {{
      let fd = ptrace::getreg!($regs, $fd_arg).try_into().unwrap();
      if let Some(fd_desc) = state.fd_allocator.get_desc_for_fd(fd) {
        let mountpoint = fd_desc.mountpoint.to_path_buf();
        fbb.reset();
        let data = syscalls::$syscall::serialize_call(&mut fbb, fd, state);
        let socket = mountsockets.get_mut(mountpoint.as_path()).unwrap();
        socket.write(data);
        let code = match syscalls::$syscall::deserialize_ret(socket.read(), fd, state) {
          Ok($ret) => $get_code,
          Err(err) => err as u64
        };
        ptrace::fake_syscall($pid, $regs, code);
      } else {
        ptrace::wait_syscall($pid).unwrap();
      }
    }};
    ($pid:expr, $regs:expr, $mod:tt, $fd_arg:tt, ret_code) => {
      route_fd!($pid, $regs, $mod, $fd_arg, |_ret| {
        _ret
      })
    };
    ($pid:expr, $regs:expr, $mod:tt, $fd_arg:tt $(, ret_data $ret_arg:tt $([$ret_len_arg:tt])?)?) => {
      route_fd!($pid, $regs, $mod, $fd_arg, |_ret| {
        $(
          let bytes = _ret.to_bytes();
          let _len = bytes.len();
          $(let _len = ptrace::getreg!($regs, $ret_len_arg) as usize;)?
          ptrace::write_bytes(pid, ptrace::getreg!($regs, $ret_arg), bytes, _len);
        )?
        0
      })
    };
  }

  wait().unwrap();
  while let Ok(_) = ptrace::wait_syscall(pid) {
    let regs = ptrace::getregs(pid).unwrap();
    match ptrace::getreg!(regs, syscall_nr) {
      ptrace::syscall_nr!(open) => route_path!(pid, regs, open, arg0, ret_code),
      ptrace::syscall_nr!(close) => route_fd!(pid, regs, close, arg0),
      ptrace::syscall_nr!(stat) => route_path!(pid, regs, stat, arg0, ret_data arg1),
      ptrace::syscall_nr!(fstat) => route_fd!(pid, regs, fstat, arg0, ret_data arg1),
      ptrace::syscall_nr!(lstat) => route_path!(pid, regs, lstat, arg0, ret_data arg1),
      ptrace::syscall_nr!(statx) => route_path!(pid, regs, statx, arg1, ret_data arg4),
      ptrace::syscall_nr!(getcwd) => route_all!(pid, regs, getcwd, ret_data arg0[arg1]),
      _ => {
        ptrace::wait_syscall(pid).unwrap();
      }
    }
  };
  Ok(())
}