extern crate includedir_codegen;

use std::env;

use includedir_codegen::Compression;

fn main() {
    let mut cg = includedir_codegen::start("FILES");
    if env::var("PASSTHROUGH").is_ok() {
        cg.passthrough();
    }
    cg.dir("../templates", Compression::Gzip)
      .build("data.rs")
      .unwrap();
}
