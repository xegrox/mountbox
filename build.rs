use std::{env, path::Path};

use flatc::flatc;

fn main() {
  println!("cargo:rerun-if-changed=flatbuffers");
  let out_dir = env::var_os("OUT_DIR").unwrap();
  flatc_rust::Flatc::from_path(flatc()).run(flatc_rust::Args {
    lang: "rust",
    inputs: &[
      Path::new("./flatbuffers/request.fbs"),
      Path::new("./flatbuffers/response.fbs")
    ],
    out_dir: &Path::new(&out_dir).join("flatbuffers"),
    ..Default::default()
  }).unwrap();

}