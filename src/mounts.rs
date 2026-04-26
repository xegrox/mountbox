use std::{collections::BTreeMap, fs::File, os::fd::{AsRawFd, OwnedFd}, path::{Path, PathBuf}, sync::Arc};
use dashmap::{mapref::one::Ref, DashMap};
use anyhow::Result;
use crate::plugin::Plugin;

pub struct FileInfo {
  pub fd: OwnedFd,
  pub fh: u64,
  pub path: PathBuf,
  pub mountpath: Arc<Path>
}

pub struct Mount {
  pub path: Arc<Path>,
  pub plugin: Arc<Plugin<'static>>,
  fds: DashMap<u16, FileInfo>,
  fd_lookup_table: Arc<DashMap<u16, Arc<Path>>>
}

impl Mount {
  pub fn get_fd_info(&self, fd: u16) -> Option<Ref<u16, FileInfo>> {
    self.fds.get(&fd)
  }

  pub fn allocate_fd(&self, path: PathBuf, fh: Option<u64>) -> Result<u16> {
    let fd = OwnedFd::from(File::open("/dev/null")?);
    let raw_fd = fd.as_raw_fd() as u16;
    self.fds.insert(raw_fd, FileInfo {
      fd,
      fh: fh.unwrap_or(0),
      path,
      mountpath: self.path.clone()
    });
    self.fd_lookup_table.insert(raw_fd, self.path.clone());
    Ok(raw_fd)
  }
}

pub struct Mounts {
  mounts: BTreeMap<Arc<Path>, Mount>,
  fd_lookup_table: Arc<DashMap<u16, Arc<Path>>>
}

impl Mounts {
  pub fn new(mounts: &[(PathBuf, Arc<Plugin<'static>>)]) -> Mounts {
    let fd_lookup_table = Arc::new(DashMap::new());
    let mounts = mounts.into_iter().map(|(pathbuf, plugin)| {
      let path = Arc::<Path>::from(pathbuf.as_path());
      (path.clone(), Mount {
        path,
        plugin: plugin.clone(),
        fds: DashMap::new(),
        fd_lookup_table: fd_lookup_table.clone()
      })
    }).collect::<BTreeMap<Arc<Path>, Mount>>();
    Mounts { mounts, fd_lookup_table }
  }

  pub fn get_mount_of_fd(&self, fd: u16) -> Option<&Mount> {
    if let Some(mountpath) = self.fd_lookup_table.get(&fd) {
      self.mounts.get(&*mountpath)
    } else {
      None
    }
  }

  pub fn get_mount_of_path(&self, path: &Path) -> Option<&Mount> {
    for (mountpath, mount) in self.mounts.iter().rev() {
      if path.starts_with(mountpath) {
        return Some(mount);
      }
    }
    None
  }

  pub fn get_mount(&self, mountpath: &Path) -> Option<&Mount> {
    self.mounts.get(mountpath)
  }
}