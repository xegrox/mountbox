use std::{io::{Read, Write}, mem::MaybeUninit};
use common::raw;
use mountbox::{syscall_nr, tracer};
use nix::libc;
use typed_path::NativePathBuf;

mod common;

create_plugin!(fstat_should_return_stat_plugin, getattr: |
  path: *const std::os::raw::c_char,
  stat: *mut raw::stat| -> std::os::raw::c_int {
    let path = unsafe { std::ffi::CStr::from_ptr(path).to_str().unwrap() };
    assert_eq!(path, "/fstat");
    let stat = unsafe { stat.as_mut().unwrap() };
    stat.mode = raw::S_IFREG;
    stat.size = 10;
    stat.atime = -10;
    stat.mtime = -10;
    stat.ctime = -10;
    return 0;
});

#[test]
fn fstat_should_return_stat() {
  let (mut r, mut w) = std::io::pipe().unwrap();
  let child = run_child!(move || {
    unsafe {
      let fd_buf = &mut [0u8; 8];
      r.read(fd_buf).unwrap();
      let fd = i64::from_ne_bytes(*fd_buf);
      let cstat = MaybeUninit::<nix::libc::stat>::zeroed().assume_init();
      let res = libc::syscall(syscall_nr!(fstat), fd, &cstat);
      assert_eq!(res, 0);
      assert_eq!(cstat.st_mode & libc::S_IFMT, libc::S_IFREG);
      assert_eq!(cstat.st_size, 10);
      assert_eq!(cstat.st_atime, -10);
      assert_eq!(cstat.st_mtime, -10);
      assert_eq!(cstat.st_ctime, -10);
    };
  });
  let state = create_state!("/test", fstat_should_return_stat_plugin);
  let mount = state.mounts.get_mount(&NativePathBuf::from("/test")).unwrap();
  let fd = mount.allocate_fd("/fstat", None).unwrap();
  w.write(&fd.to_ne_bytes()).unwrap();
  let status = tracer::attach(state.clone(), child).unwrap();
  assert_eq!(status, tracer::TraceeStatus::Exited(0));
}