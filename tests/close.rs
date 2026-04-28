use std::io::{Read, Write};
use common::raw;
use mountbox::{syscall_nr, tracer};
use nix::{fcntl::{fcntl, FcntlArg::F_GETFD}, libc};
use typed_path::PlatformPathBuf;

mod common;

create_plugin!(close_should_drop_fd_plugin, close, |path: *const std::os::raw::c_char, _fh: u64| -> std::os::raw::c_int {
  let path = unsafe { std::ffi::CStr::from_ptr(path).to_str().unwrap() };
  assert_eq!(path, "/close");
  return 0;
});

#[test]
fn close_should_drop_fd() {
  let (mut r, mut w) = std::io::pipe().unwrap();
  let child = run_child!(move || {
    unsafe {
      let fd_buf = &mut [0u8; 8];
      r.read(fd_buf).unwrap();
      let fd = i64::from_ne_bytes(*fd_buf);
      let res = libc::syscall(syscall_nr!(close), fd);
      assert!(res == 0);
    };
  });
  let state = create_state!("/test", close_should_drop_fd_plugin);
  let mount = state.mounts.get_mount(&PlatformPathBuf::from("/test")).unwrap();
  let fd = mount.allocate_fd("/close", None).unwrap();
  w.write(&fd.to_ne_bytes()).unwrap();
  assert!(fcntl(fd as i32, F_GETFD).unwrap() != -1);
  let code = tracer::attach(state.clone(), child).unwrap();
  assert_eq!(code, 0);
  assert!(mount.get_fd_info(fd).is_none());
  assert_eq!(fcntl(fd as i32, F_GETFD).err(), Some(nix::errno::Errno::EBADF));
}