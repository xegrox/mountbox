use std::{ffi::CString, mem::MaybeUninit};
use common::raw;
use mountbox::{syscall_nr, ptrace};
use nix::libc;

mod common;

create_plugin!(statx_should_return_stat_plugin, getattr, |
  path: *const std::os::raw::c_char,
  stat: *mut raw::stat| -> std::os::raw::c_int {
    let path = unsafe { std::ffi::CStr::from_ptr(path).to_str().unwrap() };
    assert_eq!(path, "/statx");
    let stat = unsafe { stat.as_mut().unwrap() };
    stat.mode = raw::S_IFREG;
    stat.size = 10;
    stat.atime = -10;
    stat.mtime = -10;
    stat.ctime = -10;
    return 0;
});

#[test]
fn statx_should_return_stat() {
  let child = run_child!(move || {
    unsafe {
      let cpath = CString::new("/test/statx").unwrap();
      let cstatx = MaybeUninit::<nix::libc::statx>::zeroed().assume_init();
      let res = libc::syscall(syscall_nr!(statx), libc::AT_FDCWD, cpath.as_ptr(), &cstatx);
      assert_eq!(res, 0);
      assert_eq!(cstatx.stx_mode as u32 & libc::S_IFMT, libc::S_IFREG);
      assert_eq!(cstatx.stx_size, 10);
      assert_eq!(cstatx.stx_atime.tv_sec, -10);
      assert_eq!(cstatx.stx_mtime.tv_sec, -10);
      assert_eq!(cstatx.stx_ctime.tv_sec, -10);
    };
  });
  let state = create_state!("/test", statx_should_return_stat_plugin);
  let code = ptrace::attach(state.clone(), child).unwrap();
  assert_eq!(code, 0);
}