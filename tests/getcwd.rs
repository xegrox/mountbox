use std::{path::Path, sync::{Arc, RwLock}};

use common::MockSocket;
use mountbox::{state::State, syscall_nr};
use nix::libc;

mod common;

#[test]
fn getcwd_should_return_cwd() {

  fn test_getcwd() {
    unsafe {
      let buf = [0u8;8];
      let res = libc::syscall(syscall_nr!(getcwd), &buf as *const _, 8);
      assert_eq!(res, 0);
      assert_eq!(String::from_utf8_lossy(&buf), "/getcwd\0");
    };
  }

  let socket = MockSocket::new();
  let mount_path = std::path::Path::new("/test");
  let mounts = mountbox::mounts::Mounts::new(vec![(mount_path, Box::new(socket))]);
  let cwd = RwLock::new(Path::new("/getcwd").to_path_buf());
  let state = Arc::new(State { mounts, cwd, ..Default::default() });
  test_syscall!(state, test_getcwd);
}