use std::{ffi::CString, fs::File, io::{BufRead, BufReader}, os::fd::{AsRawFd, FromRawFd, IntoRawFd}, sync::{OnceLock, RwLock}};
use common::raw;
use mountbox::{syscall_nr, tracer};
use nix::{libc, sys::memfd};

mod common;

static PIPE: OnceLock<(i32, i32)> = OnceLock::new();

create_plugin!(execve_noarg_noenv_should_succeed_plugin,
  open: |path: *const std::os::raw::c_char| -> std::os::raw::c_int {
    let path = unsafe { std::ffi::CStr::from_ptr(path).to_str().unwrap() };
    assert_eq!(path, "/execve");
    return 0;
  },
  read: |
    path: *const std::os::raw::c_char,
    buf: *mut std::os::raw::c_char,
    size: u64,
    offset: i64,
    _fh: u64
  | -> std::os::raw::c_int {
    let path = unsafe { std::ffi::CStr::from_ptr(path).to_str().unwrap() };
    assert_eq!(path, "/execve");
    let buf = unsafe { std::slice::from_raw_parts_mut(buf, size as usize) };
    let cmd = format!("#!/bin/sh\necho execve_success>&{}", PIPE.get().unwrap().1.as_raw_fd());
    let bin = cmd.as_bytes();
    if (offset as usize) < bin.len() {
      buf[0] = bin[offset as usize] as i8;
      return 1
    } else {
      return 0
    }
  },
  close: |path: *const std::os::raw::c_char, _fh: u64| -> std::os::raw::c_int {
    let path = unsafe { std::ffi::CStr::from_ptr(path).to_str().unwrap() };
    assert_eq!(path, "/execve");
    return 0;
  }
);

#[test]
fn execve_noarg_noenv_should_succeed() {
  let (r, _w) = PIPE.get_or_init(|| {
    let pipe = nix::unistd::pipe().unwrap();
    (pipe.0.into_raw_fd(), pipe.1.into_raw_fd())
  });
  let execve_fd = RwLock::new(memfd::memfd_create(CString::new("mountbox").unwrap().as_c_str(), memfd::MemFdCreateFlag::empty()).unwrap().into_raw_fd() as u16);
  let child: nix::unistd::Pid = run_child!(move || {
    unsafe {
      let path = CString::new("/test/execve").unwrap();
      libc::syscall(syscall_nr!(execve), path.as_ptr());
    };

  });
  let state = create_state!("/test", execve_noarg_noenv_should_succeed_plugin, {
    execve_fd
  });
  let status = tracer::attach(state.clone(), child).unwrap();
  assert_eq!(status, tracer::TraceeStatus::Exited(0));
  let mut buf = String::new();
  unsafe { assert!(libc::poll(&mut libc::pollfd {
    fd: *r,
    events: libc::POLLIN,
    revents: 0,
  }, 1, 1000) > 0, "did not receive output from execve"); }
  unsafe { BufReader::new(File::from_raw_fd(*r)).read_line(&mut buf).unwrap() };
  assert_eq!(buf, "execve_success\n");
}