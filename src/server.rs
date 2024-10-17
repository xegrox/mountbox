use nix::{sys::wait::wait, unistd::Pid};

use crate::{router, ptrace, state::State};

pub fn run(state: &mut State, pid: Pid) {
  wait().unwrap();
  while let Ok(_) = ptrace::wait_syscall(pid) {
    let regs = ptrace::getregs(pid).unwrap();
    if let Err(err) = router::route(state, regs, pid) {
      eprintln!("{}", err);
    };
  };
}