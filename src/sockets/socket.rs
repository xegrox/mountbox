pub trait Socket {
  fn write(&mut self, data: &[u8]);
  fn read(&mut self) -> Vec<u8>;
}