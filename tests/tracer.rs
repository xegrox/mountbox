use std::{ffi::CString, io::{PipeReader, PipeWriter, Read, Write}, sync::{Mutex, OnceLock}};
use common::raw;
use mountbox::{syscall_nr, tracer};
use nix::{libc, sys::signal, unistd::Pid};

mod common;

create_plugin!(tracer_plugin_error_should_cause_syscall_errno_plugin,
  open: |path: *const std::os::raw::c_char| -> std::os::raw::c_int {
    let path = unsafe { std::ffi::CStr::from_ptr(path).to_str().unwrap() };
    return match &path[1..] {
      "UNKNOWN" => i32::MIN,
      "EPERM" => -(raw::EPERM as i32),
      "ENOENT" => -(raw::ENOENT as i32),
      _ => unreachable!()
    }
  }
);

#[test]
fn tracer_plugin_error_should_cause_syscall_errno() {
  let child = run_child!(|| {
    unsafe {
      macro_rules! test_err {
        ($perr:tt, $serr:tt) => {
          let path = CString::new(format!("/test/{}", stringify!($perr))).unwrap();
          let code = libc::syscall(syscall_nr!(open), path.as_ptr());
          let errno = std::io::Error::last_os_error().raw_os_error().unwrap();
          assert_eq!(code, -1);
          assert_eq!(errno, nix::libc::$serr.into(), "unexpected errno {} for {} plugin error", errno, stringify!($perr));
        };
      }
      test_err!(UNKNOWN, EPERM);
      test_err!(EPERM, EPERM);
      test_err!(ENOENT, ENOENT);
    };
  });
  
  let state = create_state!("/test", tracer_plugin_error_should_cause_syscall_errno_plugin);
  let status = tracer::attach(state.clone(), child).unwrap();
  assert_eq!(status, tracer::TraceeStatus::Exited(0));
}

create_plugin!(tracer_syscall_invalid_addr_should_cause_syscall_efault_plugin);

#[test]
fn tracer_syscall_invalid_addr_should_cause_syscall_efault() {
  let child = run_child!(|| {
    unsafe {
      let code = libc::syscall(syscall_nr!(open), 0);
      let errno = std::io::Error::last_os_error().raw_os_error().unwrap();
      assert_eq!(code, -1);
      assert_eq!(errno, nix::libc::EFAULT);
    }
  });
  
  let state = create_state!("/test", tracer_syscall_invalid_addr_should_cause_syscall_efault_plugin);
  let status = tracer::attach(state.clone(), child).unwrap();
  assert_eq!(status, tracer::TraceeStatus::Exited(0));
}


create_plugin!(tracer_syscall_invalid_arg_should_cause_syscall_einval_plugin);

#[test]
fn tracer_syscall_invalid_arg_should_cause_syscall_einval() {
  let child = run_child!(|| {
    unsafe {
      let code = libc::syscall(syscall_nr!(open), &[0xFF]); // Invalid utf8 path
      let errno = std::io::Error::last_os_error().raw_os_error().unwrap();
      assert_eq!(code, -1);
      assert_eq!(errno, nix::libc::EINVAL);
    }
  });
  
  let state = create_state!("/test", tracer_syscall_invalid_arg_should_cause_syscall_einval_plugin);
  let status = tracer::attach(state.clone(), child).unwrap();
  assert_eq!(status, tracer::TraceeStatus::Exited(0));
}


create_plugin!(tracer_syscall_path_too_long_should_cause_syscall_enametoolong_plugin);

#[test]
fn tracer_syscall_path_too_long_should_cause_syscall_enametoolong() {
  let child = run_child!(|| {
    unsafe {
      let very_long_str = CString::new("a".repeat(nix::libc::PATH_MAX as usize)).unwrap();
      let code = libc::syscall(syscall_nr!(open), very_long_str.as_ptr()); // Invalid utf8 path
      let errno = std::io::Error::last_os_error().raw_os_error().unwrap();
      assert_eq!(code, -1);
      assert_eq!(errno, nix::libc::ENAMETOOLONG);
    }
  });
  
  let state = create_state!("/test", tracer_syscall_path_too_long_should_cause_syscall_enametoolong_plugin);
  let status = tracer::attach(state.clone(), child).unwrap();
  assert_eq!(status, tracer::TraceeStatus::Exited(0));
}


create_plugin!(tracer_child_exited_should_return_code_plugin);

#[test]
fn tracer_child_exited_should_return_code() {
  #[allow(irrefutable_let_patterns)]
  let child = run_child!(|| {
    std::process::exit(10);
  });
  
  let state = create_state!("/test", tracer_child_exited_should_return_code_plugin);
  let status = tracer::attach(state.clone(), child).unwrap();
  assert_eq!(status, tracer::TraceeStatus::Exited(10));
}

create_plugin!(tracer_child_killed_should_return_signal_plugin);

#[test]
fn tracer_child_killed_should_return_signal() {
  let child = run_child!(|| {
    signal::kill(Pid::this(), signal::SIGKILL).unwrap()
  });
  
  let state = create_state!("/test", tracer_child_killed_should_return_signal_plugin);
  let status = tracer::attach(state.clone(), child).unwrap();
  assert_eq!(status, tracer::TraceeStatus::Killed(signal::SIGKILL));
}

static PID_PIPE: OnceLock<(Mutex<PipeReader>, Mutex<PipeWriter>)> = OnceLock::new();
create_plugin!(tracer_child_killed_in_syscall_should_return_signal_plugin, open: |_: *const std::os::raw::c_char| -> std::os::raw::c_int {
  let mut buf = [0u8; 4];
  PID_PIPE.get().unwrap().0.lock().unwrap().read(&mut buf).unwrap();
  let pid = i32::from_ne_bytes(buf);
  unsafe { libc::kill(pid, libc::SIGKILL) }
});

#[test]
fn tracer_child_killed_in_syscall_should_return_signal() {
  PID_PIPE.get_or_init(|| {
    let (r, w) = std::io::pipe().unwrap();
    (Mutex::new(r), Mutex::new(w))
  });
  let child = run_child!(|| {
    let path = CString::new(format!("/test/{}", stringify!($perr))).unwrap();
    unsafe { libc::syscall(syscall_nr!(open), path.as_ptr()); }
  });
  PID_PIPE.get().unwrap().1.lock().unwrap().write(&child.as_raw().to_ne_bytes()).unwrap();
  let state = create_state!("/test", tracer_child_killed_in_syscall_should_return_signal_plugin);
  let status = tracer::attach(state.clone(), child).unwrap();
  assert_eq!(status, tracer::TraceeStatus::Killed(signal::SIGKILL));
}