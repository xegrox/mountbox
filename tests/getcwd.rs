use std::path::Path;

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

  let mut state = State { cwd: Path::new("/getcwd").to_path_buf(), ..Default::default() };
  let socket = MockSocket::new();
  test_syscall!(socket, test_getcwd, &mut state);
}