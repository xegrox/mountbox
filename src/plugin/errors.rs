use thiserror::Error;

#[derive(Error, Debug)]
pub enum PluginError {
  #[error("Unknown error")]
  UNKNOWN,
  #[error("Operation not permitted")]
  EPERM,
  #[error("No such file or directory")]
  ENOENT
}