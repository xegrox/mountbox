use std::{collections::HashMap, path::Path};
use nix::{sys::wait::wait, unistd::Pid};

use crate::{router::setup_routers, ptrace, sockets::Socket, state::State};

pub fn run<'a>(pid: Pid, mut mountsockets: HashMap<&'a Path, Box<dyn Socket>>, state: &mut State<'a>) {
  setup_routers!(state, mountsockets, pid);

  wait().unwrap();
  while let Ok(_) = ptrace::wait_syscall(pid) {
    let regs = ptrace::getregs(pid).unwrap();
    match ptrace::getreg!(regs, syscall_nr) {
      ptrace::syscall_nr!(open) => route_path!(regs, open, arg0, ret_code),
      ptrace::syscall_nr!(close) => route_fd!(regs, close, arg0),
      ptrace::syscall_nr!(stat) => route_path!(regs, stat, arg0, ret_data arg1),
      ptrace::syscall_nr!(fstat) => route_fd!(regs, fstat, arg0, ret_data arg1),
      ptrace::syscall_nr!(lstat) => route_path!(regs, lstat, arg0, ret_data arg1),
      ptrace::syscall_nr!(statx) => route_path!(regs, statx, arg1, ret_data arg4),
      ptrace::syscall_nr!(getcwd) => route_all!(regs, getcwd, ret_data arg0[arg1]),
      _ => {
        ptrace::wait_syscall(pid).unwrap();
      }
    }
  };
}