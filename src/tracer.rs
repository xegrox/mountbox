use std::{cell::RefCell, sync::Arc, thread::{self, JoinHandle}};

use nix::{sys::wait::{waitpid, WaitStatus}, unistd::Pid};

use crate::{router, ptrace, state::State};
use anyhow::{anyhow, Result};

pub fn attach(state: Arc<State>, pid: Pid) -> Result<u8> {
  ptrace::attach(pid)?;
  waitpid(pid, None)?; // TODO: support multiple tid per pid
  ptrace::setoptions(pid, ptrace::Options::PTRACE_O_TRACESYSGOOD)?;
  ptrace::syscall(pid, None)?;
  let mut threads: Vec<JoinHandle<Result<u8>>> = vec![];
  loop {
    let sig = waitpid(pid, None)?;
    match sig {
      WaitStatus::PtraceSyscall(_) => {
        let is_called = RefCell::new(false);
        let wait_ptrace_ret = || {
          *is_called.borrow_mut() = true;
          ptrace::syscall(pid, None)?;
          loop {
            let sig = waitpid(pid, None)?;
            match sig {
              WaitStatus::PtraceSyscall(_) => {
                break;
              }
              WaitStatus::Exited(_, code) => {
                return Err(anyhow!("child process exited with code {}", code)); // FIXME: safely exit
              }
              _ => {
                dbg!(sig);
              }
            }
          }
          Ok(())
        };
        let regs = ptrace::getregs(pid)?;
        if matches!(ptrace::getreg!(regs, syscall_nr), ptrace::syscall_nr!(exit_group) | ptrace::syscall_nr!(exit)) {
          for thread in threads {
            thread.join().unwrap()?;
          }
          return Ok(ptrace::getreg!(regs, arg0) as u8);
        } else if matches!(ptrace::getreg!(regs, syscall_nr), ptrace::syscall_nr!(vfork) | ptrace::syscall_nr!(fork)) {
          wait_ptrace_ret()?;
          let pid = Pid::from_raw(ptrace::getreg!(ptrace::getregs(pid)?, rax).cast_signed().try_into()?);
          let s = state.clone();
          let join = thread::spawn(move || -> Result<u8> {
            attach(s, pid)
          });
          threads.push(join);
        } else {
          router::route(&state, regs, pid, wait_ptrace_ret)?;
          assert!(*is_called.borrow(), "wait_ptrace_ret must be called");
        }
      }
      WaitStatus::Exited(_, code) => {
        return Ok(code as u8);
      }
      _ => {
        // dbg!(sig);
      }
    }
    ptrace::syscall(pid, None)?;
  }
}