use std::{os::unix::process::CommandExt, path::Path, process::{exit, Command}, sync::{Arc, RwLock}};
use anyhow::{anyhow, Result};
use mountbox::{mounts::Mounts, sockets, state::State};
use mountbox::tracer;
use nix::{libc::{raise, SIGSTOP}, unistd::{fork, ForkResult}};
use clap::Parser;

fn multipath_parser<const N: usize>(value: &str) -> Result<[String; N]> {
  value.splitn(N, ':').map(|p| {
    p.to_string()
  }).collect::<Vec<String>>().try_into().map_err(|_| anyhow!("Missing dir or socket path"))
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
  #[arg(short='u', long, value_name="DIR:SOCKET_PATH", num_args=1.., value_parser=multipath_parser::<2>)]
  bind_unix_socket: Option<Vec<[String; 2]>>,

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

      let mut mountsockets: Vec<(&Path, Box<dyn sockets::Socket>)> = vec![];
      if let Some(value) = &args.bind_unix_socket {
        for [dirp, socketp] in value {
          mountsockets.push((Path::new(dirp), Box::new(sockets::unix::UnixSocket::connect(&socketp).unwrap())));
        }
      }
      let state = Arc::new(State {
        mounts: Mounts::new(mountsockets),
        cwd: RwLock::new(std::env::current_dir().unwrap()),
        ..Default::default()
      });
      tracer::attach(state, child).unwrap();
    }
  }
}