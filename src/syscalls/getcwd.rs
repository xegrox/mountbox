use std::ffi::OsStr;

use nix::errno::Errno;

use crate::state::State;

pub fn handler<'a>(state: &'a mut State) -> Result<&'a OsStr, Errno> {
  Ok(state.cwd.as_os_str())
}