use std::{ffi::CString, str::FromStr};
use common::raw;
use mountbox::{syscall_nr, tracer};
use nix::{fcntl::readlink, libc};

mod common;

create_plugin!(chdir_plugin);

#[test]
fn chdir_to_mount_should_succeed() {
  let child = run_child!(move || {
    unsafe {
      let path = CString::from_str("/test/chdir").unwrap();
      let res = libc::syscall(syscall_nr!(chdir), path.as_ptr());
      assert!(res == 0);
    };
  });
  let state = create_state!("/test", chdir_plugin);
  let status = tracer::attach(state.clone(), child).unwrap();
  assert_eq!(status, tracer::TraceeStatus::Exited(0));
  assert_eq!(state.cwd.read().unwrap().to_str().unwrap(), "/test/chdir");
  // assert_eq!(readlink(format!("/proc/{}/cwd", child).as_str()).unwrap().to_str().unwrap(), "/test/chdir");
}


#[test]
fn chdir_to_not_mount_should_succeed() {
  let child = run_child!(move || {
    unsafe {
      let path = CString::from_str("/").unwrap();
      let res = libc::syscall(syscall_nr!(chdir), path.as_ptr());
      assert!(res == 0);
    };
  });
  let state = create_state!("/test", chdir_plugin);
  let status = tracer::attach(state.clone(), child).unwrap();
  assert_eq!(status, tracer::TraceeStatus::Exited(0));
  assert_eq!(state.cwd.read().unwrap().to_str().unwrap(), "/");
  assert_eq!(readlink(format!("/proc/{}/cwd", child).as_str()).unwrap().to_str().unwrap(), "/");
}