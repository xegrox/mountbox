use std::{ffi::CString, mem::MaybeUninit};

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

  pub fn close(&self, path: &str, fh: u64) -> Result<()> {
    let cpath = CString::new(path).unwrap();
    unsafe {
      let res = exec!(self, close, cpath.as_ptr(), fh);
      int_to_result!(res)
    }
  }

  pub fn read(&self, path: &str, buf: &mut [u8], offset: i64, fh: u64) -> Result<u64> {
    let cpath = CString::new(path).unwrap();
    unsafe {
      let res = exec!(self, read, cpath.as_ptr(), buf.as_mut_ptr() as *mut i8, buf.len() as u64, offset, fh);
      int_to_result!(res)?;
      Ok(res as u64)
    }
  }

  pub fn getattr(&self, path: &str) -> Result<raw::stat> {
    let cpath = CString::new(path).unwrap();
    unsafe {
      let mut stat = MaybeUninit::<raw::stat>::zeroed();
      let res = exec!(self, getattr, cpath.as_ptr(), stat.as_mut_ptr());
      int_to_result!(res)?;
      Ok(stat.assume_init())
    }
  }
}

