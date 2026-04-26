use std::{io::{Read, Write}, path::PathBuf};
use common::raw;
use mountbox::{syscall_nr, tracer};
use nix::libc;

mod common;

create_plugin!(read_should_return_data_plugin, read, |
  path: *const ::std::os::raw::c_char,
  buf: *mut ::std::os::raw::c_char,
  size: u64,
  _offset: i64| -> std::os::raw::c_int {
    let path = unsafe { std::ffi::CStr::from_ptr(path).to_str().unwrap() };
    let buf = unsafe { std::slice::from_raw_parts_mut(buf, size as usize) };
    assert_eq!(path, "/read");
    assert_eq!(size, 10);
    buf.fill(10);
    return 10;
});

#[test]
fn read_should_return_data() {
  let (mut r, mut w) = std::io::pipe().unwrap();
  let child = run_child!(move || {
    unsafe {
      let fd_buf = &mut [0u8; 8];
      r.read(fd_buf).unwrap();
      let fd = i64::from_ne_bytes(*fd_buf);
      let buf = &[0u8; 10];
      let len = libc::syscall(syscall_nr!(read), fd, buf, 10);
      assert!(len == 10);
    };
  });
  let state = create_state!("/test", read_should_return_data_plugin);
  let mount = state.mounts.get_mount(&PathBuf::from("/test")).unwrap();
  let fd = mount.allocate_fd(PathBuf::from("/read"), None).unwrap();
  w.write(&fd.to_ne_bytes()).unwrap();
  let code = tracer::attach(state.clone(), child).unwrap();
  assert_eq!(code, 0);
}