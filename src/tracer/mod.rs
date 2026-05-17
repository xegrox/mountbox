use std::{cell::RefCell, sync::Arc, thread};
use nix::{errno::Errno, sys::{signal, wait::{waitpid, WaitStatus}}};
use crate::{plugin, state::State};

mod ptrace;
mod router;

impl plugin::PluginError {
  fn to_errno(&self) -> i32 {
    match self {
      plugin::PluginError::UNKNOWN => nix::libc::EPERM,
      plugin::PluginError::EPERM => nix::libc::EPERM,
      plugin::PluginError::ENOENT => nix::libc::ENOENT,
    }
  }
}

macro_rules! waitpid {
  ($pid:expr $(, $body:expr)?) => {
    match waitpid($pid, None)? {
      WaitStatus::Exited(_, code) => {
        return Ok(TraceeStatus::Exited(code as u8));
      },
      WaitStatus::Signaled(_, signal, _) => {
        return Ok(TraceeStatus::Killed(signal))
      },
      $(WaitStatus::PtraceSyscall(_) => $body,)?
      _ => {}
    };
  };
}

#[derive(PartialEq, Debug)]
pub enum TraceeStatus {
  Exited(u8),
  Killed(signal::Signal)
}

pub fn attach(state: Arc<State>, pid: ptrace::Pid) -> Result<TraceeStatus, Errno> {
  let res = _attach(state, pid);
  if let Err(Errno::ESRCH) = res {
    waitpid!(pid); // Retrieve exit code
  }
  res
}

pub fn _attach(state: Arc<State>, pid: ptrace::Pid) -> Result<TraceeStatus, Errno> {
  ptrace::attach(pid)?;
  waitpid!(pid); // TODO: support multiple tid per pid
  ptrace::setoptions(pid, ptrace::Options::PTRACE_O_TRACESYSGOOD)?;
  ptrace::syscall(pid, None)?;
  let mut threads: Vec<thread::JoinHandle<Result<TraceeStatus, Errno>>> = vec![];
  loop {
    waitpid!(pid, {
      let is_called = RefCell::new(false);
      let wait_ptrace_ret = || {
        ptrace::syscall(pid, None)?;
        let sig = waitpid(pid, None)?;
        *is_called.borrow_mut() = true;
        return match sig {
          WaitStatus::PtraceSyscall(_) => Ok(()),
          WaitStatus::Exited(_, code) => Err(router::RouterError::TraceeExited(code)),
          WaitStatus::Signaled(_, signal, _) => Err(router::RouterError::TraceeKilled(signal)),
          _ => unreachable!()
        }
      };
      macro_rules! wait_ptrace_ret {
        () => {
          if let Err(err) = wait_ptrace_ret() {
            if let router::RouterError::TraceeExited(code) = err {
              return Ok(TraceeStatus::Exited(code as u8));
            } else if let router::RouterError::PtraceError(errno) = err {
              // Irrecoverable ptrace error
              return Err(errno);
            } else {
              unreachable!();
            }
          }
        };
      }
      let regs = ptrace::getregs(pid)?;
      if matches!(ptrace::getreg!(regs, syscall_nr), ptrace::syscall_nr!(exit_group) | ptrace::syscall_nr!(exit)) {
        for thread in threads {
          thread.join().unwrap()?;
        }
        return Ok(TraceeStatus::Exited(ptrace::getreg!(regs, arg0) as u8));
      } else if matches!(ptrace::getreg!(regs, syscall_nr), ptrace::syscall_nr!(vfork) | ptrace::syscall_nr!(fork)) {
        wait_ptrace_ret!();
        let pid = ptrace::Pid::from_raw(ptrace::getreg!(ptrace::getregs(pid)?, rax).cast_signed() as i32);
        let s = state.clone();
        let join = thread::spawn(move || -> Result<TraceeStatus, Errno> {
          attach(s, pid)
        });
        threads.push(join);
      } else {
        match router::route(&state, regs, pid, wait_ptrace_ret) {
          Ok(_) => {},
          Err(router::RouterError::TraceeExited(code)) => {
            return Ok(TraceeStatus::Exited(code as u8));
          },
          Err(router::RouterError::TraceeKilled(signal)) => {
            return Ok(TraceeStatus::Killed(signal))
          },
          Err(router::RouterError::PluginError(err)) => {
            dbg!(&err);
            if !*is_called.borrow() {
              // Propagate plugin error to syscall error
              wait_ptrace_ret!();
              ptrace::setregs(pid, ptrace::user_regs_struct {
                rax: -err.to_errno() as u64,
                ..ptrace::getregs(pid)?
              }).unwrap();
            }
          },
          Err(router::RouterError::PtraceError(errno)) => {
            dbg!(errno);
            if !*is_called.borrow() {
              wait_ptrace_ret!();
              ptrace::setregs(pid, ptrace::user_regs_struct {
                rax: -(errno as i64) as u64,
                ..ptrace::getregs(pid)?
              })?;
            }
          }
          Err(router::RouterError::IOError(e)) => {
            dbg!(&e);
            if !*is_called.borrow() {
              wait_ptrace_ret!();
              ptrace::setregs(pid, ptrace::user_regs_struct {
                rax: -e.raw_os_error().unwrap_or(nix::libc::EPERM) as u64,
                ..ptrace::getregs(pid)?
              }).unwrap();
            }
          }
        }
        assert!(*is_called.borrow(), "wait_ptrace_ret must be called");
      }
    });
    ptrace::syscall(pid, None)?;
  }
}