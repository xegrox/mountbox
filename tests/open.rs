use std::{ffi::CString, io::{Read, Write}, path::PathBuf};
use common::raw;
use mountbox::{syscall_nr, tracer};
use nix::{fcntl::{fcntl, FcntlArg::F_GETFD}, libc};

mod common;

create_plugin!(open_should_allocate_fd_plugin, open, |path: *const std::os::raw::c_char| -> std::os::raw::c_int {
  let path = unsafe { std::ffi::CStr::from_ptr(path).to_str().unwrap() };
  assert_eq!(path, "/open");
  return 0;
});

#[test]
fn open_should_allocate_fd() {
  let (mut r, mut w) = std::io::pipe().unwrap();
  let child = run_child!(move || {
    unsafe {
      let path = CString::new("/test/open").unwrap();
      let open_fd = libc::syscall(syscall_nr!(open), path.as_ptr());
      assert!(open_fd > 0);
      w.write(&open_fd.to_ne_bytes()).unwrap();
    };
  });
  let state = create_state!("/test", open_should_allocate_fd_plugin);
  let code = tracer::attach(state.clone(), child).unwrap();
  assert_eq!(code, 0);
  let mount = state.mounts.get_mount(&PathBuf::from("/test")).unwrap();
  let buf = &mut [0u8; 8];
  r.read(buf).unwrap();
  let fd = i64::from_ne_bytes(*buf);
  let fd_info = mount.get_fd_info(fd as u16);
  assert!(fd_info.is_some());
  assert_eq!(fd_info.unwrap().path, PathBuf::from("/open"));
  assert!(fcntl(fd as i32, F_GETFD).unwrap() != -1);
}