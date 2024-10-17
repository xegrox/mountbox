#[allow(dead_code, unused_imports)]
pub mod req {
  include!(concat!(env!("OUT_DIR"), "/flatbuffers/request_generated.rs"));
}

#[allow(dead_code, unused_imports)]
pub mod res {
  include!(concat!(env!("OUT_DIR"), "/flatbuffers/response_generated.rs"));
}