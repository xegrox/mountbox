use std::cell::RefCell;

use nix::{sys::wait::{wait, WaitStatus}, unistd::Pid};

use crate::{router, ptrace, state::State};
use anyhow::{anyhow, Result};

pub fn run(state: &mut State, pid: Pid) -> Result<u8> {
  wait()?;
  ptrace::setoptions(pid, ptrace::Options::PTRACE_O_TRACESYSGOOD)?;
  ptrace::syscall(pid, None)?;
  loop {
    let sig = wait()?;
    match sig {
      WaitStatus::PtraceSyscall(_) => {
        let regs = ptrace::getregs(pid)?;
        if matches!(ptrace::getreg!(regs, syscall_nr), ptrace::syscall_nr!(exit_group) | ptrace::syscall_nr!(exit)) {
          return Ok(ptrace::getreg!(regs, arg0) as u8);
        }
        let is_called = RefCell::new(false);
        let wait_ptrace_ret = || {
          *is_called.borrow_mut() = true;
          ptrace::syscall(pid, None)?;
          loop {
            let sig = wait()?;
            match sig {
              WaitStatus::PtraceSyscall(_) => {
                break;
              }
              WaitStatus::Exited(_, code) => {
                return Err(anyhow!("child process exited with code {}", code));
              }
              _ => {
                dbg!(sig);
              }
            }
          }
          Ok(())
        };
        router::route(state, regs, pid, wait_ptrace_ret)?;
        assert!(*is_called.borrow(), "wait_ptrace_ret must be called");
      }
      WaitStatus::Exited(_, code) => {
        return Err(anyhow!("child process exited with code {}", code));
      }
      _ => {
        dbg!(sig);
      }
    }
    ptrace::syscall(pid, None)?;
  }
}