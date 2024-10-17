use std::{collections::HashMap, os::unix::process::CommandExt, process::{exit, Command}};
use anyhow::{anyhow, Result};
use mountbox::{fd_allocator::FdAllocator, sockets, state::State};
use mountbox::ptrace;
use mountbox::server;
use nix::unistd::{fork, ForkResult};
use clap::Parser;

fn multipath_parser<const N: usize>(value: &str) -> Result<[String; N]> {
  value.splitn(N, ':').map(|p| {
    p.to_string()
  }).collect::<Vec<String>>().try_into().map_err(|_| anyhow!("Paths not specified"))
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
      ptrace::traceme().unwrap();
      let mut cmd = Command::new(&args.command[0]);
      if args.command.len() > 1 {
        cmd.args(&args.command[1..]);
      }
      cmd.exec();
      exit(0);
    }

    ForkResult::Parent { child } => {
      let mut mountsockets = HashMap::<&str, Box<dyn sockets::Socket>>::new();
      if let Some(value) = &args.bind_unix_socket {
        for [dirp, socketp] in value {
          mountsockets.insert(dirp, Box::new(sockets::unix::UnixSocket::connect(&socketp).unwrap()));
        }
      }
      server::run(child, mountsockets, &mut State {
        fd_allocator: FdAllocator::new()
      }).unwrap();
    }
  }
}