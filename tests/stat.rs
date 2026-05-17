use std::{ffi::CString, mem::MaybeUninit};
use common::raw;
use mountbox::{syscall_nr, tracer};
use nix::libc;

mod common;

create_plugin!(stat_should_return_stat_plugin, getattr: |
  path: *const std::os::raw::c_char,
  stat: *mut raw::stat| -> std::os::raw::c_int {
    let path = unsafe { std::ffi::CStr::from_ptr(path).to_str().unwrap() };
    assert_eq!(path, "/stat");
    let stat = unsafe { stat.as_mut().unwrap() };
    stat.mode = raw::S_IFREG;
    stat.size = 10;
    stat.atime = -10;
    stat.mtime = -10;
    stat.ctime = -10;
    return 0;
});

#[test]
fn stat_should_return_stat() {
  let child = run_child!(move || {
    unsafe {
      let cpath = CString::new("/test/stat").unwrap();
      let cstat = MaybeUninit::<nix::libc::stat>::zeroed().assume_init();
      let res = libc::syscall(syscall_nr!(stat), cpath.as_ptr(), &cstat);
      assert_eq!(res, 0);
      assert_eq!(cstat.st_mode & libc::S_IFMT, libc::S_IFREG);
      assert_eq!(cstat.st_size, 10);
      assert_eq!(cstat.st_atime, -10);
      assert_eq!(cstat.st_mtime, -10);
      assert_eq!(cstat.st_ctime, -10);
    };
  });
  let state = create_state!("/test", stat_should_return_stat_plugin);
  let status = tracer::attach(state.clone(), child).unwrap();
  assert_eq!(status, tracer::TraceeStatus::Exited(0));
}