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

    macro_rules! route_all {
      ($regs:expr, $mod:tt, |$ret:ident| $get_code:block) => {{
        let code = match crate::syscalls::$mod::handler($state) {
          Ok($ret) => $get_code,
          Err(err) => err as u64
        };
        crate::ptrace::fake_syscall($pid, $regs, code);
      }};
      ($regs:expr, $mod:tt, ret_code) => {
        route_all!($regs, $mod, |_ret| {
          _ret
        })
      };
      ($regs:expr, $mod:tt $d(, ret_data $ret_arg:tt $d([$ret_len_arg:tt])?)?) => {
        route_all!($regs, $mod, |_ret| {
          $d(
            let bytes = _ret.to_bytes();
            let _len = bytes.len();
            $d(let _len = crate::ptrace::getreg!($regs, $ret_len_arg) as usize;)?
            crate::ptrace::write_bytes($pid, crate::ptrace::getreg!($regs, $ret_arg), bytes, _len);
          )?
          0
        })
        // {}
      };
    }
  
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

pub use setup_routers;