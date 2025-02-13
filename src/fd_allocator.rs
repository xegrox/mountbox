use std::{collections::HashMap, fs::File, os::fd::{FromRawFd, IntoRawFd, OwnedFd}, path::Path, rc::Rc};

use anyhow::Result;

pub struct FdDesc {
  pub fd: u16,
  pub id: String,
  pub mountpoint: Rc<Path>
}

pub struct FdAllocator {
  fds: HashMap<u16, FdDesc>,
}

impl FdAllocator {

  pub fn new() -> FdAllocator {
    FdAllocator { fds: HashMap::new() }
  }

  pub fn get_desc_for_fd(&self, fd: u16) -> Option<&FdDesc> {
    self.fds.get(&fd)
  }

  pub fn allocate_fd(&mut self, mountpoint: Rc<Path>, id: &str) -> Result<u16> {
    let fd = File::open("/dev/null")?.into_raw_fd() as u16;
    self.fds.insert(fd, FdDesc {
      fd,
      id: id.to_string(),
      mountpoint
    });
    Ok(fd)
  }

  pub fn drop_fd(&mut self, fd: u16) {
    if let Some(_) = self.fds.remove(&fd) {
      unsafe { drop(OwnedFd::from_raw_fd(fd as i32)) };
    }
  }

}

impl Drop for FdAllocator {
  fn drop(&mut self) {
    for (fd, _) in self.fds.iter() {
      unsafe { drop(OwnedFd::from_raw_fd(fd.clone() as i32)) };
    }
  }
}