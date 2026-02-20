use anyhow::Result;

pub trait Socket {
  fn write(&mut self, data: &[u8]) -> Result<()>;
  fn read(&mut self) -> Result<Vec<u8>>;
}