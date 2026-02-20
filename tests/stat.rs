use std::{ffi::CString, mem::MaybeUninit, path::Path, rc::Rc};

use common::MockSocket;
use flatbuffers::FlatBufferBuilder;
use mountbox::{fb, fd_allocator::FdAllocator, state::State, syscall_nr};
use nix::libc;
use rusty_fork::rusty_fork_test;

mod common;

rusty_fork_test! {

  #[test]
  fn stat_should_return_stat() {
    fn mock_res(size: u64, file_type: fb::res::FileType) -> impl Fn(&mut flatbuffers::FlatBufferBuilder) {
      move |fbb: &mut FlatBufferBuilder| {
        let stat = fb::res::Stat::create(fbb, &fb::res::StatArgs {
          type_: file_type,
          size_: size
        });
        let res = fb::res::Response::create(fbb, &fb::res::ResponseArgs {
          payload_type: fb::res::Payload::Stat,
          payload: Some(stat.as_union_value()),
          error: None
        });
        fbb.finish(res, None);
      }
    }

    fn test_req(fstat: fb::req::Stat) {
      assert_eq!(fstat.path(), "/stat");
    }

    fn test_stat(size: u64, file_type: libc::mode_t) {
      let stat = unsafe {
        let stat: libc::stat = MaybeUninit::zeroed().assume_init();
        let path = CString::new("/test/stat").unwrap();
        let res = libc::syscall(syscall_nr!(stat), path.as_ptr(), &stat as *const _);
        assert_eq!(res, 0);
        stat
      };
      assert_eq!(stat.st_size, size as i64);
      assert_eq!(stat.st_mode & libc::S_IFMT, file_type)
    }


    let mut socket = MockSocket::new();
    queue_mock_response!(socket, stat, mock_res(24, fb::res::FileType::File), test_req);
    queue_mock_response!(socket, stat, mock_res(0, fb::res::FileType::Directory), test_req);
    test_syscall!(socket, || {
      test_stat(24, libc::S_IFREG);
      test_stat(0, libc::S_IFDIR);
    });
  }

  #[test]
  fn fstat_should_return_stat() {
    fn mock_res(size: u64, file_type: fb::res::FileType) -> impl Fn(&mut flatbuffers::FlatBufferBuilder) {
      move |fbb: &mut FlatBufferBuilder| {
        let stat = fb::res::Stat::create(fbb, &fb::res::StatArgs {
          type_: file_type,
          size_: size
        });
        let res = fb::res::Response::create(fbb, &fb::res::ResponseArgs {
          payload_type: fb::res::Payload::Stat,
          payload: Some(stat.as_union_value()),
          error: None
        });
        fbb.finish(res, None);
      }
    }

    fn test_req(fstat: fb::req::Fstat) {
      assert_eq!(fstat.fd().id(), "fstat_id");
    }

    fn test_fstat(fd: u16, size: u64, file_type: libc::mode_t) {
      let stat = unsafe {
        let stat: libc::stat = MaybeUninit::zeroed().assume_init();
        let res = libc::syscall(syscall_nr!(fstat), fd as u32, &stat as *const _);
        assert_eq!(res, 0);
        stat
      };
      assert_eq!(stat.st_size, size as i64);
      assert_eq!(stat.st_mode & libc::S_IFMT, file_type)
    }


    let mut fd_allocator = FdAllocator::new();
    let fstat_fd = fd_allocator.allocate_fd(Rc::from(Path::new("/test")), "fstat_id").unwrap();
    let mut state = State { fd_allocator, ..Default::default() };

    let mut socket = MockSocket::new();
    queue_mock_response!(socket, fstat, mock_res(24, fb::res::FileType::File), test_req);
    queue_mock_response!(socket, fstat, mock_res(0, fb::res::FileType::Directory), test_req);
    test_syscall!(socket, || {
      test_fstat(fstat_fd, 24, libc::S_IFREG);
      test_fstat(fstat_fd, 0, libc::S_IFDIR);
    }, &mut state);
  }

  #[test]
  fn lstat_should_return_stat() {
    fn mock_res(size: u64, file_type: fb::res::FileType) -> impl Fn(&mut flatbuffers::FlatBufferBuilder) {
      move |fbb: &mut FlatBufferBuilder| {
        let stat = fb::res::Stat::create(fbb, &fb::res::StatArgs {
          type_: file_type,
          size_: size
        });
        let res = fb::res::Response::create(fbb, &fb::res::ResponseArgs {
          payload_type: fb::res::Payload::Stat,
          payload: Some(stat.as_union_value()),
          error: None
        });
        fbb.finish(res, None);
      }
    }

    fn test_req(fstat: fb::req::Stat) {
      assert_eq!(fstat.path(), "/lstat");
    }

    fn test_lstat(size: u64, file_type: libc::mode_t) {
      let stat = unsafe {
        let stat: libc::stat = MaybeUninit::zeroed().assume_init();
        let path = CString::new("/test/lstat").unwrap();
        let res = libc::syscall(syscall_nr!(lstat), path.as_ptr(), &stat as *const _);
        assert_eq!(res, 0);
        stat
      };
      assert_eq!(stat.st_size, size as i64);
      assert_eq!(stat.st_mode & libc::S_IFMT, file_type)
    }


    let mut socket = MockSocket::new();
    queue_mock_response!(socket, stat, mock_res(24, fb::res::FileType::File), test_req);
    queue_mock_response!(socket, stat, mock_res(0, fb::res::FileType::Directory), test_req);
    test_syscall!(socket, || {
      test_lstat(24, libc::S_IFREG);
      test_lstat(0, libc::S_IFDIR);
    });
  }

  #[test]
  fn statx_should_return_stat() {
    fn mock_res(size: u64, file_type: fb::res::FileType) -> impl Fn(&mut flatbuffers::FlatBufferBuilder) {
      move |fbb: &mut FlatBufferBuilder| {
        let stat = fb::res::Stat::create(fbb, &fb::res::StatArgs {
          type_: file_type,
          size_: size
        });
        let res = fb::res::Response::create(fbb, &fb::res::ResponseArgs {
          payload_type: fb::res::Payload::Stat,
          payload: Some(stat.as_union_value()),
          error: None
        });
        fbb.finish(res, None);
      }
    }

    fn test_req(fstat: fb::req::Stat) {
      assert_eq!(fstat.path(), "/statx");
    }

    fn test_statx(size: u64, file_type: libc::mode_t) {
      let statx = unsafe {
        let statx: libc::statx = MaybeUninit::zeroed().assume_init();
        let path = CString::new("/test/statx").unwrap();
        let res = libc::syscall(syscall_nr!(statx), libc::AT_FDCWD, path.as_ptr(), 0, libc::STATX_MODE, &statx as *const _);
        assert_eq!(res, 0);
        statx
      };
      assert_eq!(statx.stx_size, size);
      assert_eq!(statx.stx_mode & libc::S_IFMT as u16, file_type as u16)
    }

    let mut socket = MockSocket::new();
    queue_mock_response!(socket, stat, mock_res(24, fb::res::FileType::File), test_req);
    queue_mock_response!(socket, stat, mock_res(0, fb::res::FileType::Directory), test_req);
    test_syscall!(socket, || {
      test_statx(24, libc::S_IFREG);
      test_statx(0, libc::S_IFDIR);
    });

  }

}