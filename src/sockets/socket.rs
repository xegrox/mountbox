use anyhow::Result;

pub trait Socket: Sync + Send {
  fn write(&mut self, data: &[u8]) -> Result<()>;
  fn read(&mut self) -> Result<Vec<u8>>;
}