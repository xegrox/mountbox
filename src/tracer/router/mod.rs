mod close;
mod fstat;
mod lstat;
mod open;
mod read;
mod stat;
mod statx;
mod getcwd;
mod chdir;
mod execve;

use crate::{dirfd_resolver, plugin, state::State};
use super::ptrace;
use nix::{libc::user_regs_struct, unistd::Pid};
use thiserror::Error;

macro_rules! el {
  ($e:stmt, $t:tt) => {};
  ($e:stmt,) => {$e};
}

#[derive(Error, Debug)]
pub enum RouterError {
  #[error("ptrace error")]
  PtraceError(#[from] nix::errno::Errno),
  #[error("plugin error")]
  PluginError(#[from] plugin::PluginError),
  #[error("io error")]
  IOError(#[from] std::io::Error),
  #[error("child exited")]
  TraceeExited(i32),
  #[error("child killed")]
  TraceeKilled(nix::sys::signal::Signal)
}

pub type Result<T> = std::result::Result<T, RouterError>;

pub fn route<'a>(state: &State, regs: user_regs_struct, tid: Pid, wait_ptrace_ret: impl Fn() -> Result<()>) -> Result<()> {
  macro_rules! route_path {
    ($path_arg:tt $(@$dirfd_arg:tt)?, $body:expr $(, $($extra_args:expr),*)?) => {{
      let cwd = state.cwd.read().unwrap();
      let raw_path = ptrace::read_path(tid, ptrace::getreg!(regs, $path_arg))?;
      $(
        let dirfd = ptrace::getreg!(regs, $dirfd_arg) as i32;
        let fullpath = cwd.join(dirfd_resolver::resolve(tid, dirfd, &raw_path));
      )?
      el!(let fullpath = cwd.join(raw_path), $($dirfd_arg)?);
      let mount = state.mounts.get_mount_of_path(fullpath.as_path());
      if let Some(mount) = mount {
        if let Ok(relpath) = typed_path::Utf8UnixPath::from_bytes_path(fullpath.strip_prefix(&mount.path).unwrap()) {
          let path = typed_path::Utf8UnixPathBuf::from("/").join(relpath);
          $body(mount, &path, tid, regs, wait_ptrace_ret $(, $($extra_args),*)?)?;
        } else {
          return Err(RouterError::PtraceError(nix::errno::Errno::EINVAL));
        }
      } else {
        wait_ptrace_ret()?;
      }
    }};
  }

  macro_rules! route_fd {
    ($fd_arg:tt, $body:expr) => {{
      let raw_fd = ptrace::getreg!(regs, $fd_arg) as u16;
      if let Some(mount) = state.mounts.get_mount_of_fd(raw_fd) {
        $body(mount, raw_fd, tid, regs, wait_ptrace_ret)?;
      } else {
        wait_ptrace_ret()?;
      }
    }};
  }
  
  match ptrace::getreg!(regs, syscall_nr) {
    ptrace::syscall_nr!(open) => route_path!(arg0, open::open),
    ptrace::syscall_nr!(read) => route_fd!(arg0, read::read),
    ptrace::syscall_nr!(close) => route_fd!(arg0, close::close),
    ptrace::syscall_nr!(stat) => route_path!(arg0, stat::stat),
    ptrace::syscall_nr!(lstat) => route_path!(arg0, lstat::lstat),
    ptrace::syscall_nr!(fstat) => route_fd!(arg0, fstat::fstat),
    ptrace::syscall_nr!(statx) => route_path!(arg1@arg0, statx::statx),
    ptrace::syscall_nr!(getcwd) => getcwd::getcwd(state, tid, regs, wait_ptrace_ret)?,
    ptrace::syscall_nr!(chdir) => chdir::chdir(state, tid, regs, wait_ptrace_ret)?,
    ptrace::syscall_nr!(execve) => route_path!(arg0, execve::execve, &state.execve_fd),
    _ => wait_ptrace_ret()?
  }
  Ok(())
}