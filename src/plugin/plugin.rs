use std::ffi::CString;

use anyhow::{anyhow, Result};
use dlopen::symbor::{Library, Symbol};

use super::raw;

pub struct Plugin<'a> {
  raw_operations: Symbol<'a, &'a raw::mountbox_operations>
}

macro_rules! exec {
  ($self:tt, $op:tt $(, $($args:expr),*)?) => {
    ($self.raw_operations.$op.unwrap_or_else(|| unimplemented!()))($($($args),*)?)
  };
}

macro_rules! int_to_result {
  ($int:tt) => {
    if $int < 0 {
      // TODO: map errors
      Err(anyhow!("error"))
    } else {
      Ok(())
    }
  };
}

impl<'a> Plugin<'a> {
  pub fn load(lib: &'a Library, symbol_name: Option<&str>) -> Plugin<'a> {
    let raw_operations = unsafe {
      lib.symbol::<&raw::mountbox_operations>(symbol_name.unwrap_or("operations")).unwrap()
    };
    Plugin { raw_operations }
  }

  pub fn open(&self, path: &str) -> Result<()> {
    let cpath = CString::new(path).unwrap();
    unsafe {
      let res = exec!(self, open, cpath.as_ptr());
      int_to_result!(res)
    }
  }

  pub fn read(&self, path: &str, size: u64, offset: i64) -> Result<Vec<u8>> {
    let mut buf = vec![0; size as usize];
    let cpath = CString::new(path).unwrap();
    unsafe {
      let res = exec!(self, read, cpath.as_ptr(), buf.as_mut_ptr() as *mut i8, size, offset);
      int_to_result!(res)?;
      buf.resize(res as usize, 0);
      Ok(buf)
    }
  }
}

