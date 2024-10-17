use std::{collections::HashMap, path::Path, rc::Rc};

use crate::sockets::Socket;

pub struct Mount {
  pub path: Rc<Path>,
  pub socket: Box<dyn Socket>
}

pub struct Mounts {
  mountpoints: Vec<Rc<Path>>,
  mounts: HashMap<Rc<Path>, Mount>
}

impl Mounts {
  pub fn new(mounts: HashMap<&Path, Box<dyn Socket>>) -> Mounts {
    let mounts = mounts.into_iter().map(|(p, socket)| {
      let path = Rc::<Path>::from(p);
      (path.clone(), Mount { path: path.clone(), socket })
    }).collect::<HashMap<Rc<Path>, Mount>>();
    let mut mountpoints = mounts.keys().map(|p| p.clone()).collect::<Vec<Rc<Path>>>();
    mountpoints.sort_unstable_by(|a, b| { a.cmp(b) });
    Mounts { mountpoints, mounts }
  }

  pub fn get_mount_of_path(&self, path: &Path) -> Option<&Mount> {
    for mountp in self.mountpoints.iter() {
      if path.starts_with(mountp) {
        return Some(self.mounts.get(mountp).unwrap());
      }
    }
    None
  }

  pub fn get_mount_mut(&mut self, path: &Path) -> Option<&mut Mount> {
    self.mounts.get_mut(path)
  }

}