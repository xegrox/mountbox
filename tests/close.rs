use std::{path::Path, rc::Rc};

use common::MockSocket;
use mountbox::{fb, fd_allocator::FdAllocator, state::State, syscall_nr};
use nix::{fcntl::fcntl, libc};

mod common;

#[test]
fn close_should_close_fd() {

  fn test_req(close: fb::req::Close) {
    assert_eq!(close.fd().id(), "test_id");
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

  fn test_close(fd: u16) -> impl Fn() {
    move || {
      unsafe {
        let res = libc::syscall(syscall_nr!(close), fd as libc::c_uint);
        assert_eq!(res, 0);
      };
    }
  }
  
  let mut fd_allocator = FdAllocator::new();
  let fd = fd_allocator.allocate_fd(Rc::from(Path::new("/test")), "test_id").unwrap();
  let mut state = State { fd_allocator, ..Default::default() };
  
  let mut socket = MockSocket::new();
  queue_mock_response!(socket, close, mock_res, test_req);
  test_syscall!(socket, test_close(fd), &mut state);
  assert!(state.fd_allocator.get_desc_for_fd(fd).is_none(), "fd still stored in fdallocator");
  assert_eq!(fcntl(fd.into(), nix::fcntl::FcntlArg::F_GETFL), Err(nix::errno::Errno::EBADF), "fd not closed");
}