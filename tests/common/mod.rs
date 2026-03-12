use std::collections::VecDeque;
use anyhow::Result;
use mountbox::{fb, sockets::Socket};

pub struct MockSocket<'a> {
  pair: VecDeque<(Box<dyn Fn(&[u8]) + Send + Sync + 'a>, Box<dyn Fn() -> Vec<u8> + Send + Sync + 'a>)>
}

impl<'a> MockSocket<'a> {
  pub fn new() -> MockSocket<'a> {
    MockSocket { pair: VecDeque::new() }
  }

  #[allow(unused)]
  pub fn queue_pair<R, T>(&mut self, req: R, res: T)
  where
    R: Fn(&[u8]) + Send + Sync + 'a,
    T: Fn() -> Vec<u8> + Send + Sync + 'a {
    self.pair.push_back((Box::new(req), Box::new(res)));
  }
}

impl Drop for MockSocket<'_> {
  fn drop(&mut self) {
    assert!(self.pair.is_empty(), "{} unhandled mock reply", self.pair.len());
  }
}

impl Socket for MockSocket<'_> {
  fn write(&mut self, data: &[u8]) -> Result<()> {
    if let Some((res, _)) = self.pair.front() {
      Ok(res(data))
    } else {
      let req = flatbuffers::root::<fb::req::Request>(data).unwrap();
      panic!("Unhandled request {}", req.operation_type().variant_name().unwrap())
    }
  }

  fn read(&mut self) -> Result<Vec<u8>> {
    if let Some((_, req)) = self.pair.pop_front() {
      Ok(req())
    } else {
      unreachable!()
    }
  }
}

#[macro_export]
macro_rules! queue_mock_response {
  ($socket:expr, $req_type:tt, $res:expr $(, $req:expr)?) => {
    $socket.queue_pair(|_data| {
      $(let req = flatbuffers::root::<mountbox::fb::req::Request>(_data).unwrap();
      paste::paste! {
        let op = req.[<operation_as_ $req_type>]()
          .expect(&format!("Expected {} operation, got {}", stringify!($req_type), req.operation_type().variant_name().unwrap()));
      }
      $req(op);)?
    }, move || {
      let fbb = &mut flatbuffers::FlatBufferBuilder::new();
      $res(fbb);
      fbb.finished_data().to_vec()
    });
  };
}

#[macro_export]
macro_rules! test_syscall {
  ($state:expr, $test_syscall:expr) => {
    match unsafe { nix::unistd::fork().unwrap() } {
      nix::unistd::ForkResult::Child => {
        unsafe { nix::libc::raise(nix::libc::SIGSTOP); }
        if let Err(_) = std::panic::catch_unwind($test_syscall) {
          std::process::exit(101);
        } else {
          std::process::exit(0);
        }
      }
  
      nix::unistd::ForkResult::Parent { child } => {
        let code = mountbox::tracer::attach($state, child).unwrap();
        assert!(code != 101, "panic in syscall test");
      }
    }
  };
}