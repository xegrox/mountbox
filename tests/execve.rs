use std::ffi::CString;
use common::MockSocket;
use mountbox::{fb, syscall_nr};
use nix::libc;

mod common;

#[test]
fn execve_noarg_noenv_should_succeed() {

  fn test_execve() {
    unsafe {
      let path = CString::new("/test/execve").unwrap();
      libc::syscall(syscall_nr!(execve), path.as_ptr(), 0, 0);
    };
  }

  fn test_open_req(open: fb::req::Open) {
    assert_eq!(open.path(), "/execve");
  }

  fn mock_open_res(fbb: &mut flatbuffers::FlatBufferBuilder) {
    let fb_fd_id = fbb.create_string("fd_execve");
    let fb_fd = fb::res::Fd::create(fbb, &fb::res::FdArgs {
      id: Some(fb_fd_id)
    });
    let fb_req = fb::res::Response::create(fbb, &fb::res::ResponseArgs {
      payload_type: fb::res::Payload::Fd,
      payload: Some(fb_fd.as_union_value()),
      error: None
    });
    fbb.finish(fb_req, None);
  }

  fn test_read_req(read: fb::req::Read) {
    assert_eq!(read.fd().id(), "fd_execve");
  }

  fn mock_read_res(fbb: &mut flatbuffers::FlatBufferBuilder) {
    let script = String::from("#!/bin/sh\necho hello");
    let fb_data = fbb.create_vector(script.as_bytes());
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

  fn mock_read_res_eof(fbb: &mut flatbuffers::FlatBufferBuilder) {
    let fb_data = fbb.create_vector(&[0u8; 0]);
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

  fn mock_fstat_res(fbb: &mut flatbuffers::FlatBufferBuilder) {
    let fb_stat = fb::res::Stat::create(fbb, &fb::res::StatArgs {
      size_: 0,
      type_: fb::res::FileType::File
    });
    let res = fb::res::Response::create(fbb, &fb::res::ResponseArgs {
      payload_type: fb::res::Payload::Stat,
      payload: Some(fb_stat.as_union_value()),
      error: None
    });
    fbb.finish(res, None);
  }

  fn mock_close_res(fbb: &mut flatbuffers::FlatBufferBuilder) {
    let res = fb::res::Response::create(fbb, &fb::res::ResponseArgs {
      payload_type: fb::res::Payload::NONE,
      payload: None,
      error: None
    });
    fbb.finish(res, None);
  }

  let mut socket = MockSocket::new();
  queue_mock_response!(socket, open, mock_open_res, test_open_req);
  queue_mock_response!(socket, read, mock_read_res, test_read_req);
  queue_mock_response!(socket, read, mock_read_res_eof, test_read_req);
  queue_mock_response!(socket, fstat, mock_fstat_res);
  queue_mock_response!(socket, close, mock_close_res);
  test_syscall!(socket, test_execve);
}