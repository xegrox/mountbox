use std::ffi::CStr;
use common::raw;
use mountbox::{syscall_nr, tracer};
use nix::libc;
use typed_path::NativePathBuf;

mod common;

create_plugin!(getcwd_should_return_cwd_plugin);

#[test]
fn getcwd_should_return_cwd() {
  let child = run_child!(move || {
    unsafe {
      let buf = [0u8; 20];
      let res = libc::syscall(syscall_nr!(getcwd), &buf, buf.len());
      assert!(res == 0);
      let cwd = CStr::from_bytes_until_nul(&buf).unwrap();
      assert_eq!(cwd.to_str().unwrap(), "/getcwd");
    };
  });
  let state = create_state!("/test", getcwd_should_return_cwd_plugin);
  *state.cwd.write().unwrap() = NativePathBuf::from("/getcwd");
  let status = tracer::attach(state.clone(), child).unwrap();
  assert_eq!(status, tracer::TraceeStatus::Exited(0));
}