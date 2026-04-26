mod open;
mod read;

use crate::{ptrace, state::State};
use anyhow::Result;
use nix::{libc::user_regs_struct, unistd::Pid};


macro_rules! el {
  ($e:stmt, $t:tt) => {};
  ($e:stmt,) => {$e};
}

pub fn route<'a>(state: &State, regs: user_regs_struct, tid: Pid, wait_ptrace_ret: impl Fn() -> Result<()>) -> Result<()> {
  macro_rules! route_path {
    ($path_arg:tt $(@$dirfd_arg:tt)?, $body:expr) => {{
      let cwd = state.cwd.read().unwrap();
      let raw_path = crate::ptrace::read_str(tid, ptrace::getreg!(regs, $path_arg))?;
      $(
        let dirfd = ptrace::getreg!(regs, $dirfd_arg) as i32;
        let fullpath = cwd.join(state.dirfd_resolver.resolve(tid, dirfd, raw_path));
      )?
      el!(let fullpath = cwd.join(raw_path), $($dirfd_arg)?);
      let mount = state.mounts.get_mount_of_path(fullpath.as_path());
      if let Some(mount) = mount {
        let path = std::path::Path::new("/").join(fullpath.strip_prefix(&mount.path).unwrap());
        $body(mount, &path, tid, regs, wait_ptrace_ret)?;
      } else {
        wait_ptrace_ret()?;
      }
    }};
  }

  macro_rules! route_fd {
    ($fd_arg:tt, $body:expr) => {{
      let raw_fd = ptrace::getreg!(regs, $fd_arg) as u16;
      if let Some(mount) = state.mounts.get_mount_of_fd(raw_fd) {
        let fd = mount.get_fd_info(raw_fd).unwrap();
        $body(mount, &fd.path, tid, regs, wait_ptrace_ret)?;
      } else {
        wait_ptrace_ret()?;
      }
    }};
  }
  
  match ptrace::getreg!(regs, syscall_nr) {
    ptrace::syscall_nr!(open) => route_path!(arg0, open::open),
    ptrace::syscall_nr!(read) => route_fd!(arg0, read::read),
    _ => wait_ptrace_ret()?
    // ptrace::syscall_nr!(read) => route_fd!(read, arg0, args(arg2: usize), result(bytes arg1), result_code=true),
    // ptrace::syscall_nr!(open) => route_path!(open, arg0, result_code=true),
    // ptrace::syscall_nr!(openat) => route_path!(open, arg1@arg0, result_code=true),
    // ptrace::syscall_nr!(close) => route_fd!(close, arg0),
    // ptrace::syscall_nr!(stat) => route_path!(stat, arg0, result(raw arg1)),
    // ptrace::syscall_nr!(fstat) => route_fd!(fstat, arg0, result(raw arg1)),
    // ptrace::syscall_nr!(lstat) => route_path!(lstat, arg0, result(raw arg1)),
    // ptrace::syscall_nr!(statx) => route_path!(statx, arg1, result(raw arg4)),
    // ptrace::syscall_nr!(getcwd) => route_all!(getcwd, result(string arg0[arg1])),
    // ptrace::syscall_nr!(execve) => route_path_custom!(execve, arg0),
    // _ => {
    //   wait_ptrace_ret()?;
    // }
  }
  Ok(())
}