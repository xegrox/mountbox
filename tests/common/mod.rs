pub mod raw {
  #![allow(warnings)]
  include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

pub static LIB: std::sync::LazyLock<dlopen::symbor::Library> = std::sync::LazyLock::new(|| {dlopen::symbor::Library::open_self().unwrap()});


impl raw::mountbox_operations {
  pub const fn default() -> Self {
    Self {
      open: None,
      read: None
    }
  }
}

#[macro_export]
macro_rules! create_plugin {
  ($name:tt, $op:tt, |$($k:tt: $v:ty),*| -> $ret:ty {$($body:tt)*}) => { paste::paste! {

    unsafe extern "C" fn [<$name _op>]($($k:$v),*) -> $ret {$($body)*}

    #[unsafe(no_mangle)]
    #[allow(non_upper_case_globals)]
    pub static mut $name: raw::mountbox_operations = raw::mountbox_operations {
      $op: Some([<$name _op>]),
      ..$crate::common::raw::mountbox_operations::default()
    };
  } }
}

#[macro_export]
macro_rules! run_child {
  ($syscall:expr) => {
    match unsafe { nix::unistd::fork().unwrap() } {
      nix::unistd::ForkResult::Child => {
        unsafe { nix::libc::raise(nix::libc::SIGSTOP); }
        if let Err(_) = std::panic::catch_unwind($syscall) {
          std::process::exit(101);
        } else {
          std::process::exit(0);
        }
      }
  
      nix::unistd::ForkResult::Parent { child } => {
        child
      }
    }
  }
}

#[macro_export]
macro_rules! create_state {
  ($path:expr, $plugin:expr $(, {$($k:tt: $v:expr),*})?) => {
    std::sync::Arc::new(mountbox::state::State {
      mounts: mountbox::mounts::Mounts::new(&[(std::path::PathBuf::from($path), std::sync::Arc::new(mountbox::plugin::Plugin::load(&common::LIB, Some(stringify!($plugin)))))]),
      $($($k: $v),*, )?
      ..Default::default()
    })
  };
}