use nix::errno::Errno;

use crate::state::State;

pub fn handler(state: &State) -> Result<String, Errno> {
  Ok(state.cwd.read().unwrap().clone().into_os_string().into_string().unwrap())
}