use std::{io::{Read, Write}, os::unix::net::UnixStream};

use super::socket::Socket;

pub struct UnixSocket {
  _stream: UnixStream
}

impl UnixSocket {
  pub fn connect(address: &str) -> Result<UnixSocket, std::io::Error> {
    Ok(UnixSocket { _stream: UnixStream::connect(address)? })
  }
}

impl Socket for UnixSocket {
  fn write(&mut self, data: &[u8]) {
    self._stream.write(data).unwrap();
  }

  fn read(&mut self) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    self._stream.read_to_end(&mut buf).unwrap();
    buf
  }
}