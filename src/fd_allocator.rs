use std::{collections::HashMap, fs::File, os::fd::{FromRawFd, IntoRawFd, OwnedFd}};

use anyhow::Result;

pub struct FdDesc<'a> {
  pub fd: u16,
  pub id: String,
  pub mountpoint: &'a str
}

pub struct FdAllocator<'a> {
  fds: HashMap<u16, FdDesc<'a>>,
  current_mountpoint: Option<&'a str>
}

impl<'a> FdAllocator<'a> {

  pub fn new() -> FdAllocator<'a> {
    FdAllocator { fds: HashMap::new(), current_mountpoint: None }
  }

  pub fn get_desc_for_fd(&self, fd: u16) -> Option<&FdDesc> {
    self.fds.get(&fd)
  }

  pub fn set_current_mountpoint(&mut self, mountpoint: &'a str) {
    self.current_mountpoint = Some(mountpoint);
  }

  pub fn allocate_fd(&mut self, id: &str) -> Result<u16> {
    let fd = File::open("/dev/null")?.into_raw_fd() as u16;
    self.fds.insert(fd, FdDesc {
      fd,
      id: id.to_string(),
      mountpoint: self.current_mountpoint.expect("current mountpoint not set")
    });
    Ok(fd)
  }

  pub fn drop_fd(&mut self, fd: u16) {
    if let Some(_) = self.fds.remove(&fd) {
      unsafe { drop(OwnedFd::from_raw_fd(fd as i32)) };
    }
  }

}

impl<'a> Drop for FdAllocator<'a> {
    fn drop(&mut self) {
      for (fd, _) in self.fds.iter() {
        unsafe { drop(OwnedFd::from_raw_fd(fd.clone() as i32)) };
      }
    }
}