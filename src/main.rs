use std::{os::unix::process::CommandExt, process::{exit, Command, ExitCode}, sync::{Arc, OnceLock, RwLock}};
use anyhow::{anyhow, Result};
use dlopen::symbor::Library;
use mountbox::{mounts::Mounts, plugin::Plugin, tracer, state::State};
use nix::{libc, unistd::{fork, ForkResult, Pid}};
use clap::Parser;
use typed_path::NativePathBuf;

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

fn main() -> ExitCode {
  let args = Cli::parse();

  match unsafe { fork().unwrap() } {
    ForkResult::Child => {
      unsafe { libc::raise(libc::SIGSTOP) };
      let mut cmd = Command::new(&args.command[0]);
      if args.command.len() > 1 {
        cmd.args(&args.command[1..]);
      }
      let _ = cmd.exec();
      exit(0);
    }

    ForkResult::Parent { child } => {
      let mut mountsockets: Vec<(NativePathBuf, Arc<Plugin>)> = vec![];
      if let Some(value) = &args.bind {
        for [dirp, plugin_path] in value {
          static LIB: OnceLock<Library> = OnceLock::new();
          LIB.get_or_init(|| Library::open(plugin_path).unwrap());
          let plugin = Arc::new(Plugin::load(&LIB.get().unwrap(), None));
          mountsockets.push((NativePathBuf::from(dirp), plugin));
        }
      }
      let state = Arc::new(State {
        mounts: Mounts::new(&mountsockets),
        cwd: RwLock::new(NativePathBuf::from(std::env::current_dir().unwrap().as_os_str().as_encoded_bytes())),
        ..Default::default()
      });
      match tracer::attach(state, child).unwrap() {
        tracer::TraceeStatus::Exited(code) => ExitCode::from(code),
        tracer::TraceeStatus::Killed(signal) => {
          nix::sys::signal::kill(Pid::from_raw(0), signal).unwrap();
          unreachable!();
        },
      }
    }
  }
}