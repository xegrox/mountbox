use crate::{ptrace, state::State, syscalls};
use anyhow::Result;
use nix::{libc::user_regs_struct, unistd::Pid};

macro_rules! el {
    ($e:stmt, $t:tt) => {};
    ($e:stmt,) => {$e};
}

fn raw_to_bytes<T: Sized>(raw: &T) -> &[u8] {
  unsafe {
    ::core::slice::from_raw_parts(
      (raw as *const T) as *const u8,
      ::core::mem::size_of::<T>(),
    )
  }
}

macro_rules! to_bytes {
    ($a:expr, raw) => {
      raw_to_bytes($a)
    };
    ($a:expr, bytes) => {
      $a
    };
    ($a:expr, string) => {
      $a.as_bytes()
    };
}

pub fn route<'a>(state: &State, regs: user_regs_struct, tid: Pid, wait_ptrace_ret: impl Fn() -> Result<()>) -> Result<()> {

  macro_rules! bool {
    (true, $body:block, $_:block) => {
      $body
    };
    (false, $__:block, $body:block) => {
      $body
    };
    (, $__:block, $body:block) => {
      $body
    };
  }

  macro_rules! route_path_custom {
    ($syscall:tt, $path_arg:tt$(@$dirfd_arg:tt)? $(, args($($arg:tt: $type:tt)*))?) => {
      if let Ok(path) = crate::ptrace::read_str(tid, ptrace::getreg!(regs, $path_arg)) {
        $(
          let dirfd = ptrace::getreg!(regs, $dirfd_arg) as i32;
          let fullpath = state.dirfd_resolver.resolve(dirfd, &path).join(state.cwd.join(path));
        )?
        el!(let fullpath = state.cwd.read().unwrap().join(path), $($dirfd_arg)?);
        let mount = state.mounts.get_mount_of_path(fullpath.as_path());
        if let Some(mount) = mount {
          let relpath = std::path::Path::new("/").join(fullpath.strip_prefix(&mount.path).unwrap());
          let mountpoint = mount.path.clone();
          syscalls::$syscall::handler(state, tid, mountpoint, &relpath, wait_ptrace_ret, $(, $(parse_arg!($arg[$type])),*)?)?;
        } else {
          wait_ptrace_ret()?;
        }
      }
    }
  }
  
  macro_rules! route_path {
    ($mod:tt, $path_arg:tt$(@$dirfd_arg:tt)? $(, args($($arg:tt: $type:tt)*))? $(, result($res_type:tt $res_arg:tt $([$res_len_arg:tt])?))?  $(, result_code=$res_has_code:tt)?) => {{
      if let Ok(path) = crate::ptrace::read_str(tid, ptrace::getreg!(regs, $path_arg)) {
        let cwd = state.cwd.read().unwrap();
        $(
          let dirfd = ptrace::getreg!(regs, $dirfd_arg) as i32;
          let fullpath = state.dirfd_resolver.resolve(tid, dirfd, &path).join(cwd.join(path));
        )?
        el!(let fullpath = cwd.join(path), $($dirfd_arg)?);
        let mount = state.mounts.get_mount_of_path(fullpath.as_path());
        if let Some(mount) = mount {
          let relpath = std::path::Path::new("/").join(fullpath.strip_prefix(&mount.path).unwrap());
          let mountpoint = mount.path.clone();
          syscalls::$mod::serialize_req(state, mountpoint.clone(), &relpath $(, $(parse_arg!($arg[$type])),*)?)?;
          let mut socket_guard = state.mounts.get_mount(&mountpoint).unwrap().socket.lock().unwrap();
          socket_guard.write(state.fbb.lock().unwrap().finished_data())?;
          let response = socket_guard.read()?;
          drop(socket_guard);
          let code = match syscalls::$mod::deserialize_res(state, mountpoint.clone(), &relpath, &response) {
            Ok(res) => {
              let (_res, code) = bool!($($res_has_code)?, {
                res
              }, {
                (res, 0)
              });
              $(
                let bytes = to_bytes!(&_res, $res_type);
                $(let len = ptrace::getreg!(regs, $res_len_arg) as usize;)?
                el!(let len = bytes.len(), $($res_len_arg)?);
                ptrace::write_bytes(tid, ptrace::getreg!(regs, $res_arg), bytes, len);
              )?
              code
            },
            Err(err) => err as u64
          };
          ptrace::setregs(tid, user_regs_struct {
            orig_rax: u64::MAX,
            ..regs
          }).unwrap();
          wait_ptrace_ret()?;
          ptrace::setregs(tid, user_regs_struct {
            rax: code,
            ..ptrace::getregs(tid).unwrap()
          }).unwrap();
        } else {
          wait_ptrace_ret()?;
        }
      }
    }};
  }

  macro_rules! route_fd {
    ($syscall:tt, $fd_arg:tt $(, args($($arg:tt: $type:tt)*))? $(, result($res_type:tt $res_arg:tt $([$res_len_arg:tt])?))? $(, result_code=$res_has_code:tt)?) => {{
      let fd = ptrace::getreg!(regs, $fd_arg).try_into().unwrap();
      let fd_alloc_guard = state.fd_allocator.read().unwrap();
      if let Some(fd_desc) = fd_alloc_guard.get_desc_for_fd(fd) {
        let mountpoint = fd_desc.mountpoint.clone();
        drop(fd_alloc_guard);
        syscalls::$syscall::serialize_req(state, mountpoint.clone(), fd $(, $(parse_arg!($arg[$type])),*)?)?;
        let mut socket_guard = state.mounts.get_mount(&mountpoint).unwrap().socket.lock().unwrap();
        socket_guard.write(state.fbb.lock().unwrap().finished_data())?;
        let data = socket_guard.read()?;
        drop(socket_guard);
        let code = match syscalls::$syscall::deserialize_res(state, mountpoint.clone(), fd, &data) {
          Ok(res) => {
            let (_res, code) = bool!($($res_has_code)?, {
              res
            }, {
              (res, 0)
            });
            $(
              let bytes = to_bytes!(&_res, $res_type);
              $(let len = ptrace::getreg!(regs, $res_len_arg) as usize;)?
              el!(let len = bytes.len(), $($res_len_arg)?);
              ptrace::write_bytes(tid, ptrace::getreg!(regs, $res_arg), bytes, len);
            )?
            code
          },
          Err(err) => err as u64
        };
        ptrace::setregs(tid, user_regs_struct {
          orig_rax: u64::MAX,
          ..regs
        }).unwrap();
        wait_ptrace_ret()?;
        ptrace::setregs(tid, user_regs_struct {
          rax: code,
          ..ptrace::getregs(tid).unwrap()
        }).unwrap();
      } else {
        wait_ptrace_ret().unwrap();
      }
    }};
  }

  macro_rules! route_all {
    ($mod:tt $(, args($($arg:tt: $type:tt)*))? $(, result($res_type:tt $res_arg:tt $([$res_len_arg:tt])?))? $(, result_code=$res_has_code:tt)?) => {{
      let code = match syscalls::$mod::handler(state$(, $(parse_arg!($arg[$type])),*)?) {
        Ok(res) => {
          let (_res, code) = bool!($($res_has_code)?, {
            res
          }, {
            (res, 0)
          });
          $(
            let bytes = to_bytes!(&_res, $res_type);
            $(let len = ptrace::getreg!(regs, $res_len_arg) as usize;)?
            el!(let len = bytes.len(), $($res_len_arg)?);
            ptrace::write_bytes(tid, ptrace::getreg!(regs, $res_arg), bytes, len);
          )?
          code
        },
        Err(err) => err as u64
      };


      ptrace::setregs(tid, user_regs_struct {
        orig_rax: u64::MAX,
        ..regs
      }).unwrap();
      wait_ptrace_ret()?;
      ptrace::setregs(tid, user_regs_struct {
        rax: code,
        ..ptrace::getregs(tid).unwrap()
      }).unwrap();
    }};
  }

  macro_rules! parse_arg {
    ($arg:tt[usize]) => {
      usize::try_from(ptrace::getreg!(regs, $arg)).unwrap()
    };
  }
  
  match ptrace::getreg!(regs, syscall_nr) {
    ptrace::syscall_nr!(read) => route_fd!(read, arg0, args(arg2: usize), result(bytes arg1), result_code=true),
    ptrace::syscall_nr!(open) => route_path!(open, arg0, result_code=true),
    ptrace::syscall_nr!(openat) => route_path!(open, arg1@arg0, result_code=true),
    ptrace::syscall_nr!(close) => route_fd!(close, arg0),
    ptrace::syscall_nr!(stat) => route_path!(stat, arg0, result(raw arg1)),
    ptrace::syscall_nr!(fstat) => route_fd!(fstat, arg0, result(raw arg1)),
    ptrace::syscall_nr!(lstat) => route_path!(lstat, arg0, result(raw arg1)),
    ptrace::syscall_nr!(statx) => route_path!(statx, arg1, result(raw arg4)),
    ptrace::syscall_nr!(getcwd) => route_all!(getcwd, result(string arg0[arg1])),
    ptrace::syscall_nr!(execve) => route_path_custom!(execve, arg0),
    _ => {
      wait_ptrace_ret()?;
    }
  }
  Ok(())
}