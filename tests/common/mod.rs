use std::collections::VecDeque;

use mountbox::{fb, sockets::Socket};

pub struct MockSocket {
  pair: VecDeque<(Box<dyn Fn(&[u8])>, Box<dyn Fn() -> Vec<u8>>)>
}

impl MockSocket {
  pub fn new() -> MockSocket {
    MockSocket { pair: VecDeque::new() }
  }

  pub fn queue_pair<R, T>(&mut self, req: R, res: T)
  where
    R: Fn(&[u8]) + 'static,
    T: Fn() -> Vec<u8> + 'static {
    self.pair.push_back((Box::new(req), Box::new(res)));
  }
}

impl Drop for MockSocket {
  fn drop(&mut self) {
    assert!(self.pair.is_empty(), "{} unhandled mock reply", self.pair.len())
  }
}

impl Socket for MockSocket {
  fn write(&mut self, data: &[u8]) {
    if let Some((res, _)) = self.pair.front() {
      res(data)
    } else {
      let req = flatbuffers::root::<fb::req::Request>(data).unwrap();
      panic!("Unhandled request {}", req.operation_type().variant_name().unwrap())
    }
  }

  fn read(&mut self) -> Vec<u8> {
    if let Some((_, req)) = self.pair.pop_front() {
      req()
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
    }, || {
      let fbb = &mut flatbuffers::FlatBufferBuilder::new();
      $res(fbb);
      fbb.finished_data().to_vec()
    });
  };
}

#[macro_export]
macro_rules! test_syscall {
  ($socket:expr, $test_syscall:expr $(, $state:expr)?) => {
    let _s = &mut mountbox::state::State { ..Default::default() };
    $(let _s = $state;)?
  
    let socket: Box<dyn mountbox::sockets::Socket> = Box::new($socket);
    match unsafe { nix::unistd::fork().unwrap() } {
      nix::unistd::ForkResult::Child => {
        mountbox::ptrace::traceme().unwrap();
        unsafe { nix::libc::raise(nix::libc::SIGTRAP); }
        $test_syscall();
        std::process::exit(0);
      }
  
      nix::unistd::ForkResult::Parent { child } => {
        let mounts = mountbox::mounts::Mounts::new(std::collections::HashMap::from([(std::path::Path::new("/test"), socket)]));
        _s.mounts = mounts;
        mountbox::server::run(_s, child);
      }
    }
  };
}