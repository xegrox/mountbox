use std::{path::Path, rc::Rc};

use common::MockSocket;
use mountbox::{fb, fd_allocator::FdAllocator, state::State, syscall_nr};
use nix::libc;

mod common;


#[test]
fn read_returns_data() {

  fn test_req(read: fb::req::Read) {
    assert_eq!(read.fd().id(), "test_id");
    assert_eq!(read.len(), 4);
  }

  fn mock_res(fbb: &mut flatbuffers::FlatBufferBuilder) {
    let fb_data = fbb.create_vector(&[0u8, 1, 2, 3]);
    let fb_read = fb::res::Read::create(fbb, &fb::res::ReadArgs {
      data: Some(fb_data)
    });
    let res = fb::res::Response::create(fbb, &fb::res::ResponseArgs {
      payload_type: fb::res::Payload::Read,
      payload: Some(fb_read.as_union_value()),
      error: None
    });
    fbb.finish(res, None);
  }

  fn test_read(fd: u16) -> impl Fn() {
    move || {
      unsafe {
        let buf = [0u8; 4];
        let res = libc::syscall(syscall_nr!(read), fd as libc::c_uint, &buf as *const _, 4);
        assert_eq!(res, 4);
        assert_eq!(buf, [0, 1, 2, 3])
      };
    }
  }

  let mut fd_allocator = FdAllocator::new();
  let fd = fd_allocator.allocate_fd(Rc::from(Path::new("/test")), "test_id").unwrap();
  let mut state = State { fd_allocator, ..Default::default() };
  let mut socket = MockSocket::new();
  queue_mock_response!(socket, read, mock_res, test_req);
  test_syscall!(socket, test_read(fd), &mut state);
}