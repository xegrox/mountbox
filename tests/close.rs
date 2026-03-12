use std::{path::Path, sync::{Arc, RwLock}};
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
  
  let fd_allocator = RwLock::new(FdAllocator::new());
  let fd = fd_allocator.write().unwrap().allocate_fd(Arc::from(Path::new("/test")), "test_id").unwrap();
  
  let mut socket = MockSocket::new();
  queue_mock_response!(socket, close, mock_res, test_req);
  
  let mount_path = std::path::Path::new("/test");
  let mounts = mountbox::mounts::Mounts::new(vec![(mount_path, Box::new(socket))]);
  let state = Arc::new(State { mounts, fd_allocator, ..Default::default() });
  test_syscall!(state.clone(), test_close(fd));
  assert!(state.fd_allocator.read().unwrap().get_desc_for_fd(fd).is_none(), "fd still stored in fdallocator");
  assert_eq!(fcntl(fd.into(), nix::fcntl::FcntlArg::F_GETFL), Err(nix::errno::Errno::EBADF), "fd not closed");
}