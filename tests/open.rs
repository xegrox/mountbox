use std::ffi::CString;

use common::MockSocket;
use mountbox::{fb, syscall_nr};
use nix::libc;

mod common;

#[test]
fn open_should_allocate_fd() {

  fn test_req(open: fb::req::Open) {
    assert_eq!(open.path(), "/open");
  }

  fn mock_res(fbb: &mut flatbuffers::FlatBufferBuilder) {
    let fb_id = Some(fbb.create_string("test_id"));
    let fb_fd = fb::res::Fd::create(fbb, &fb::res::FdArgs {
      id: fb_id
    });
    let res = fb::res::Response::create(fbb, &fb::res::ResponseArgs {
      payload_type: fb::res::Payload::Fd,
      payload: Some(fb_fd.as_union_value()),
      error: None
    });
    fbb.finish(res, None);
  }

  fn test_open() {
    unsafe {
      let path = CString::new("/test/open").unwrap();
      let open_fd = libc::syscall(syscall_nr!(open), path.clone());
      assert!(open_fd > 0);
    };
  }

  fn test_openat() {
    unsafe {
      let path = CString::new("/test/open").unwrap();
      let openat_fd = libc::syscall(syscall_nr!(openat), libc::AT_FDCWD, path);
      assert!(openat_fd > 0);
    }
  }

  let mut socket = MockSocket::new();
  queue_mock_response!(socket, open, mock_res, test_req);
  queue_mock_response!(socket, open, mock_res, test_req);
  test_syscall!(socket, || {
    test_open();
    test_openat();
  });
}