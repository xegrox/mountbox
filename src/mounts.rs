use std::{collections::HashMap, path::Path, sync::{Arc, Mutex}};

use crate::sockets::Socket;

pub struct Mount {
  pub path: Arc<Path>,
  pub socket: Mutex<Box<dyn Socket>>
}

pub struct Mounts {
  mountpoints: Vec<Arc<Path>>,
  mounts: HashMap<Arc<Path>, Mount>
}

impl Mounts {
  pub fn new(mounts: Vec<(&Path, Box<dyn Socket>)>) -> Mounts {
    let mounts = mounts.into_iter().map(|(p, s)| {
      let path = Arc::<Path>::from(p);
      let socket = Mutex::new(s);
      (path.clone(), Mount { path: path.clone(), socket })
    }).collect::<HashMap<Arc<Path>, Mount>>();
    let mut mountpoints = mounts.keys().map(|p| p.clone()).collect::<Vec<Arc<Path>>>();
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

  pub fn get_mount(&self, path: &Path) -> Option<&Mount> {
    self.mounts.get(path)
  }

}