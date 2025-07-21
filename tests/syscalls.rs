use std::collections::HashMap;
use std::ffi::CString;
use std::mem::MaybeUninit;
use std::process::exit;
use std::path::Path;
use std::rc::Rc;
use mountbox::fb;
use mountbox::{state::State, sockets::Socket, ptrace, server, syscall_nr, fd_allocator::FdAllocator, mounts::Mounts};
use nix::libc;
use nix::unistd::{fork, ForkResult};
use rusty_fork::rusty_fork_test;

macro_rules! test_syscall {
    (
      $test_syscall:expr,
      $parse_req:expr,
      $mock_res:expr
      $(, $state: expr)?
    ) => {{
      struct MockSocket {}

      impl Socket for MockSocket {
        fn write(&mut self, data: &[u8]) {
          $parse_req(data);
        }
      
        fn read(&mut self) -> Vec<u8> {
          $mock_res()
        }
      }
    
      let socket: Box<dyn Socket> = Box::new(MockSocket {});
      match unsafe { fork().unwrap() } {
        ForkResult::Child => {
          ptrace::traceme().unwrap();
          unsafe { libc::raise(libc::SIGTRAP); }
          $test_syscall();
          exit(0);
        }
    
        ForkResult::Parent { child } => {
          let mounts = Mounts::new(HashMap::from([(Path::new("/test"), socket)]));
          let _s = &mut State {
            fd_allocator: FdAllocator::new(),
            ..Default::default()
          };
          $(let _s = $state;)?
          _s.mounts = mounts;
          server::run(_s, child);
        }
      }
    }};
}

rusty_fork_test! {

  #[test]
  fn stat() {
    let mut fd_allocator = FdAllocator::new();
    let fstat_fd = fd_allocator.allocate_fd(Rc::from(Path::new("/test")), "fstat_id").unwrap();
    let mut state = State { fd_allocator, ..Default::default() };

    fn parse_req(data: &[u8]) {
      let req = flatbuffers::root::<fb::req::Request>(data).unwrap();
      let op = req.operation_as_stat().expect("Expected stat operation");
      assert_eq!(op.path(), "/stat");
    }

    fn parse_req_fstat(data: &[u8]) {
      let req = flatbuffers::root::<fb::req::Request>(data).unwrap();
      let op = req.operation_as_fstat().expect("Expected fstat operation");
      assert_eq!(op.fd().id(), "fstat_id");
    }

    fn mock_res(size: u64, file_type: fb::res::FileType) -> impl Fn() -> Vec<u8> {
      move || {
        let fbb = &mut flatbuffers::FlatBufferBuilder::new();
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
        fbb.finished_data().to_vec()
      }
    }

    fn test_statx(size: u64, file_type: libc::mode_t) -> impl Fn() {
      move || {
        let statx = unsafe {
          let statx: libc::statx = MaybeUninit::zeroed().assume_init();
          let path = CString::new("/test/stat").unwrap();
          let res = libc::syscall(syscall_nr!(statx), libc::AT_FDCWD, path.as_ptr(), 0, libc::STATX_MODE, &statx as *const _);
          assert_eq!(res, 0);
          statx
        };
        assert_eq!(statx.stx_size, size);
        assert_eq!(statx.stx_mode & libc::S_IFMT as u16, file_type as u16)
      }
    }

    fn test_stat(size: u64, file_type: libc::mode_t) -> impl Fn() {
      move || {
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
    }

    fn test_lstat(size: u64, file_type: libc::mode_t) -> impl Fn() {
      move || {
        let stat = unsafe {
          let stat: libc::stat = MaybeUninit::zeroed().assume_init();
          let path = CString::new("/test/stat").unwrap();
          let res = libc::syscall(syscall_nr!(lstat), path.as_ptr(), &stat as *const _);
          assert_eq!(res, 0);
          stat
        };
        assert_eq!(stat.st_size, size as i64);
        assert_eq!(stat.st_mode & libc::S_IFMT, file_type)
      }
    }

    fn test_fstat(fd: u16, size: u64, file_type: libc::mode_t) -> impl Fn() {
      move || {
        let stat = unsafe {
          let stat: libc::stat = MaybeUninit::zeroed().assume_init();
          let res = libc::syscall(syscall_nr!(fstat), fd as u32, &stat as *const _);
          assert_eq!(res, 0);
          stat
        };
        assert_eq!(stat.st_size, size as i64);
        assert_eq!(stat.st_mode & libc::S_IFMT, file_type)
      }
    }

    test_syscall!(test_statx(24, libc::S_IFREG), parse_req, mock_res(24, fb::res::FileType::File));
    test_syscall!(test_statx(0, libc::S_IFDIR), parse_req, mock_res(0, fb::res::FileType::Directory));
    test_syscall!(test_stat(24, libc::S_IFREG), parse_req, mock_res(24, fb::res::FileType::File));
    test_syscall!(test_stat(0, libc::S_IFDIR), parse_req, mock_res(0, fb::res::FileType::Directory));
    test_syscall!(test_lstat(24, libc::S_IFREG), parse_req, mock_res(24, fb::res::FileType::File));
    test_syscall!(test_lstat(0, libc::S_IFDIR), parse_req, mock_res(0, fb::res::FileType::Directory));

    test_syscall!(test_fstat(fstat_fd, 24, libc::S_IFREG), parse_req_fstat, mock_res(24, fb::res::FileType::File), &mut state);
    test_syscall!(test_fstat(fstat_fd, 0, libc::S_IFDIR), parse_req_fstat, mock_res(0, fb::res::FileType::Directory), &mut state);
  }

  #[test]
  fn open() {
    fn parse_req(data: &[u8]) {
      let req = flatbuffers::root::<fb::req::Request>(data).unwrap();
      let op = req.operation_as_open().expect("Expected open operation");
      assert_eq!(op.path(), "/open");
    }

    fn mock_res() -> Vec<u8> {
      let fbb = &mut flatbuffers::FlatBufferBuilder::new();
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
      fbb.finished_data().to_vec()
    }

    fn test_open() -> impl Fn() {
      move || {
        unsafe {
          let path = CString::new("/test/open").unwrap();
          let fd = libc::syscall(syscall_nr!(open), path);
          assert!(fd > 0);
        };
      }
    }

    fn test_openat() -> impl Fn() {
      move || {
        unsafe {
          let path = CString::new("/test/open").unwrap();
          let fd = libc::syscall(syscall_nr!(openat), libc::AT_FDCWD, path);
          assert!(fd > 0);
        };
      }
    }

    test_syscall!(test_open(), parse_req, mock_res);
    test_syscall!(test_openat(), parse_req, mock_res);
  }

  #[test]
  fn close() {
    
    fn parse_req(data: &[u8]) {
      let req = flatbuffers::root::<fb::req::Request>(data).unwrap();
      let op = req.operation_as_close().expect("Expected close operation");
      assert_eq!(op.fd().id(), "test_id");
    }

    fn mock_res() -> Vec<u8> {
      let fbb = &mut flatbuffers::FlatBufferBuilder::new();
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
      fbb.finished_data().to_vec()
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
    test_syscall!(test_close(fd), parse_req, mock_res, &mut state);
    assert!(state.fd_allocator.get_desc_for_fd(fd).is_none());
  }

  #[test]
  fn read() {

    fn parse_req(data: &[u8]) {
      let req = flatbuffers::root::<fb::req::Request>(data).unwrap();
      let op = req.operation_as_read().expect("Expected read operation");
      assert_eq!(op.fd().id(), "test_id");
      assert_eq!(op.len(), 4);
    }

    fn mock_res() -> Vec<u8> {
      let fbb = &mut flatbuffers::FlatBufferBuilder::new();
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
      fbb.finished_data().to_vec()
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
    test_syscall!(test_read(fd), parse_req, mock_res, &mut state);
  }
  
  #[test]
  fn getcwd() {

    fn parse_req(_: &[u8]) {
      unreachable!()
    }

    fn mock_res() -> Vec<u8> {
      unreachable!()
    }

    fn test_getcwd() {
      unsafe {
        let buf = [0u8;8];
        let res = libc::syscall(syscall_nr!(getcwd), &buf as *const _, 8);
        assert_eq!(res, 0);
        assert_eq!(String::from_utf8_lossy(&buf), "/getcwd\0");
      };
    }

    let mut state = State { cwd: Path::new("/getcwd").to_path_buf(), ..Default::default() };
    test_syscall!(test_getcwd, parse_req, mock_res, &mut state);
  }
}