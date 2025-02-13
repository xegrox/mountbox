use crate::{ptrace, state::State};
use nix::{libc::user_regs_struct, unistd::Pid};

trait ToBytes {
  fn to_bytes(&self) -> &[u8];
}

impl ToBytes for std::ffi::OsStr {
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

pub fn route<'a>(state: &mut State, regs: user_regs_struct, pid: Pid) {
  
  macro_rules! route_path {
    ($mod:tt, $path_arg:tt, |$ret:ident| $get_code:block) => {{
      if let Ok(path) = crate::ptrace::read_str(pid, crate::ptrace::getreg!(regs, $path_arg)) {
        let fullpath = state.cwd.join(path);
        let mount = state.mounts.get_mount_of_path(fullpath.as_path());
        if let Some(mount) = mount {
          state.fbb.reset();
          let relpath = std::path::Path::new("/").join(fullpath.strip_prefix(&mount.path).unwrap());
          let mountpoint = mount.path.clone();
          crate::syscalls::$mod::serialize_req(state, mountpoint.clone(), &relpath);
          let socket = &mut state.mounts.get_mount_mut(&mountpoint).unwrap().socket;
          socket.write(state.fbb.finished_data());
          let response = socket.read();
          let code = match crate::syscalls::$mod::deserialize_res(state, mountpoint.clone(), &relpath, response) {
            Ok($ret) => $get_code,
            Err(err) => err as u64
          };
          crate::ptrace::fake_syscall(pid, regs, code);
        } else {
          crate::ptrace::wait_syscall(pid).unwrap();
        }
      }
    }};
    ($mod:tt, $path_arg:tt, ret_code) => {
      route_path!($mod, $path_arg, |_ret| {
        _ret
      })
    };
    ($mod:tt, $path_arg:tt $(, ret_data $ret_arg:tt $([$ret_len_arg:tt])?)?) => {
      route_path!($mod, $path_arg, |_ret| {
        $(
          let bytes = _ret.to_bytes();
          let _len = bytes.len();
          $(let _len = crate::ptrace::getreg!($regs, $ret_len_arg) as usize;)?
          crate::ptrace::write_bytes(pid, crate::ptrace::getreg!(regs, $ret_arg), bytes, _len);
        )?
        0
      })
    };
  }

  macro_rules! route_fd {
    ($syscall:tt, $fd_arg:tt, |$ret:ident| $get_code:block) => {{
      let fd = crate::ptrace::getreg!(regs, $fd_arg).try_into().unwrap();
      if let Some(fd_desc) = state.fd_allocator.get_desc_for_fd(fd) {
        let mountpoint = fd_desc.mountpoint.clone();
        state.fbb.reset();
        crate::syscalls::$syscall::serialize_req(state, mountpoint.clone(), fd);
        let socket = &mut state.mounts.get_mount_mut(&mountpoint).unwrap().socket;
        socket.write(state.fbb.finished_data());
        let res = socket.read();
        let code = match crate::syscalls::$syscall::deserialize_res(state, mountpoint.clone(), fd, res) {
          Ok($ret) => $get_code,
          Err(err) => err as u64
        };
        crate::ptrace::fake_syscall(pid, regs, code);
      } else {
        crate::ptrace::wait_syscall(pid).unwrap();
      }
    }};
    ($mod:tt, $fd_arg:tt, ret_code) => {
      route_fd!($mod, $fd_arg, |_ret| {
        _ret
      })
    };
    ($mod:tt, $fd_arg:tt $(, ret_data $ret_arg:tt $([$ret_len_arg:tt])?)?) => {
      route_fd!($mod, $fd_arg, |_ret| {
        $(
          let bytes = _ret.to_bytes();
          let _len = bytes.len();
          $(let _len = crate::ptrace::getreg!($regs, $ret_len_arg) as usize;)?
          crate::ptrace::write_bytes(pid, crate::ptrace::getreg!(regs, $ret_arg), bytes, _len);
        )?
        0
      })
    };
  }

  macro_rules! route_direct {
    ($mod:tt, |$ret:ident| $get_code:block) => {{
      let code = match crate::syscalls::$mod::handler(state) {
        Ok($ret) => $get_code,
        Err(err) => err as u64
      };
      crate::ptrace::fake_syscall(pid, regs, code);
    }};
    ($mod:tt, ret_code) => {
      route_direct!($mod, |_ret| {
        _ret
      })
    };
    ($mod:tt $(, ret_data $ret_arg:tt $([$ret_len_arg:tt])?)?) => {
      route_direct!($mod, |_ret| {
        $(
          let bytes = _ret.to_bytes();
          let _len = bytes.len();
          $(let _len = crate::ptrace::getreg!(regs, $ret_len_arg) as usize;)?
          crate::ptrace::write_bytes(pid, crate::ptrace::getreg!(regs, $ret_arg), bytes, _len);
        )?
        0
      })
    };
  }

  match ptrace::getreg!(regs, syscall_nr) {
    ptrace::syscall_nr!(open) => route_path!(open, arg0, ret_code),
    ptrace::syscall_nr!(close) => route_fd!(close, arg0),
    ptrace::syscall_nr!(stat) => route_path!(stat, arg0, ret_data arg1),
    ptrace::syscall_nr!(fstat) => route_fd!(fstat, arg0, ret_data arg1),
    ptrace::syscall_nr!(lstat) => route_path!(lstat, arg0, ret_data arg1),
    ptrace::syscall_nr!(statx) => route_path!(statx, arg1, ret_data arg4),
    ptrace::syscall_nr!(getcwd) => route_direct!(getcwd, ret_data arg0[arg1]),
    _ => {
      ptrace::wait_syscall(pid).unwrap();
    }
  }
}

#[macro_export]
macro_rules! setup_routers {
  ($state:expr, $mountsockets:expr, $pid:expr) => {
    setup_routers!(@internal $state, $mountsockets, $pid, $)
  };
  (@internal $state:expr, $mountsockets:expr, $pid:expr, $d:tt) => {
    trait ToBytes {
      fn to_bytes(&self) -> &[u8];
    }
    
    impl ToBytes for std::ffi::OsStr {
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

    let mut fbb = flatbuffers::FlatBufferBuilder::new();
    let mut mountpoints = $mountsockets.iter().map(|(&p, _)| p).collect::<Vec<&std::path::Path>>();
    mountpoints.sort_unstable_by(|a, b| { a.cmp(b) });

    let get_parent_mountpoint = |path: &std::path::Path| -> Option<&std::path::Path> {
      for mountp in mountpoints.iter() {
        if path.starts_with(mountp) {
          return Some(mountp);
        }
      }
      None
    };

    
  
    macro_rules! route_path {
      ($regs:expr, $mod:tt, $path_arg:tt, |$ret:ident| $get_code:block) => {{
        if let Ok(path) = crate::ptrace::read_str($pid, crate::ptrace::getreg!($regs, $path_arg)) {
          let fullpath = $state.cwd.join(path);
          let mountp = get_parent_mountpoint(fullpath.as_path());
          if let Some(mountp) = mountp {
            fbb.reset();
            let relpath = std::path::Path::new("/").join(fullpath.strip_prefix(mountp).unwrap());
            let data = crate::syscalls::$mod::serialize_call(&relpath, &mut fbb, $state);
            let socket = $mountsockets.get_mut(&mountp).unwrap();
            socket.write(data);
            $state.fd_allocator.set_current_mountpoint(mountp);
            let code = match crate::syscalls::$mod::deserialize_ret(&relpath, socket.read(), $state) {
              Ok($ret) => $get_code,
              Err(err) => err as u64
            };
            crate::ptrace::fake_syscall($pid, $regs, code);
          } else {
            crate::ptrace::wait_syscall($pid).unwrap();
          }
        }
      }};
      ($regs:expr, $mod:tt, $path_arg:tt, ret_code) => {
        route_path!($regs, $mod, $path_arg, |_ret| {
          _ret
        })
      };
      ($regs:expr, $mod:tt, $path_arg:tt $d(, ret_data $ret_arg:tt $d([$ret_len_arg:tt])?)?) => {
        route_path!($regs, $mod, $path_arg, |_ret| {
          $d(
            let bytes = _ret.to_bytes();
            let _len = bytes.len();
            $d(let _len = crate::ptrace::getreg!($regs, $ret_len_arg) as usize;)?
            crate::ptrace::write_bytes($pid, crate::ptrace::getreg!($regs, $ret_arg), bytes, _len);
          )?
          0
        })
      };
    }
  
    macro_rules! route_fd {
      ($regs:expr, $syscall:tt, $fd_arg:tt, |$ret:ident| $get_code:block) => {{
        let fd = crate::ptrace::getreg!($regs, $fd_arg).try_into().unwrap();
        if let Some(fd_desc) = $state.fd_allocator.get_desc_for_fd(fd) {
          let mountpoint = fd_desc.mountpoint.to_path_buf();
          fbb.reset();
          let data = crate::syscalls::$syscall::serialize_call(&mut fbb, fd, $state);
          let socket = $mountsockets.get_mut(mountpoint.as_path()).unwrap();
          socket.write(data);
          let code = match crate::syscalls::$syscall::deserialize_ret(socket.read(), fd, $state) {
            Ok($ret) => $get_code,
            Err(err) => err as u64
          };
          crate::ptrace::fake_syscall($pid, $regs, code);
        } else {
          crate::ptrace::wait_syscall($pid).unwrap();
        }
      }};
      ($regs:expr, $mod:tt, $fd_arg:tt, ret_code) => {
        route_fd!($regs, $mod, $fd_arg, |_ret| {
          _ret
        })
      };
      ($regs:expr, $mod:tt, $fd_arg:tt $d(, ret_data $ret_arg:tt $d([$ret_len_arg:tt])?)?) => {
        route_fd!($regs, $mod, $fd_arg, |_ret| {
          $d(
            let bytes = _ret.to_bytes();
            let _len = bytes.len();
            $d(let _len = crate::ptrace::getreg!($regs, $ret_len_arg) as usize;)?
            crate::ptrace::write_bytes($pid, crate::ptrace::getreg!($regs, $ret_arg), bytes, _len);
          )?
          0
        })
      };
    }
  };
}
