use std::{os::unix::process::CommandExt, process::{exit, Command}, sync::{Arc, OnceLock, RwLock}};
use anyhow::{anyhow, Result};
use dlopen::symbor::Library;
use mountbox::{mounts::Mounts, plugin::Plugin, ptrace, state::State};
use nix::{libc::{raise, SIGSTOP}, unistd::{fork, ForkResult}};
use clap::Parser;
use typed_path::PlatformPathBuf;

fn multipath_parser<const N: usize>(value: &str) -> Result<[String; N]> {
  value.splitn(N, ':').map(|p| {
    p.to_string()
  }).collect::<Vec<String>>().try_into().map_err(|_| anyhow!("Missing dir or socket path"))
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
  #[arg(short='u', long, value_name="DIR:PLUGIN_PATH", num_args=1.., value_parser=multipath_parser::<2>)]
  bind: Option<Vec<[String; 2]>>,

  #[arg(last = true, required = true)]
  command: Vec<String>
}

fn main() {
  let args = Cli::parse();

  match unsafe { fork().unwrap() } {
    ForkResult::Child => {
      unsafe { raise(SIGSTOP) };
      let mut cmd = Command::new(&args.command[0]);
      if args.command.len() > 1 {
        cmd.args(&args.command[1..]);
      }
      let _ = cmd.exec();
      exit(0);
    }

    ForkResult::Parent { child } => {
      let mut mountsockets: Vec<(PlatformPathBuf, Arc<Plugin>)> = vec![];
      if let Some(value) = &args.bind {
        for [dirp, plugin_path] in value {
          static LIB: OnceLock<Library> = OnceLock::new();
          LIB.get_or_init(|| Library::open(plugin_path).unwrap());
          let plugin = Arc::new(Plugin::load(&LIB.get().unwrap(), None));
          mountsockets.push((PlatformPathBuf::from(dirp), plugin));
        }
      }
      let state = Arc::new(State {
        mounts: Mounts::new(&mountsockets),
        cwd: RwLock::new(PlatformPathBuf::from(std::env::current_dir().unwrap().as_os_str().as_encoded_bytes())),
        ..Default::default()
      });
      ptrace::attach(state, child).unwrap();
    }
  }
}