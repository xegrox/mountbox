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
    let size_prefix = (data.len() as u32).to_be_bytes();
    self._stream.write(&size_prefix).unwrap();
    self._stream.write_all(data).unwrap();
  }

  fn read(&mut self) -> Vec<u8> {
    let mut size_buf = [0u8; 4];
    while self._stream.read_exact(&mut size_buf).is_err_and(|e| e.kind() == std::io::ErrorKind::UnexpectedEof) {}
    let size = u32::from_be_bytes(size_buf) as usize;
    let mut buf: Vec<u8> = vec![0; size];
    self._stream.read_exact(&mut buf[..size]).unwrap();
    buf
  }
}