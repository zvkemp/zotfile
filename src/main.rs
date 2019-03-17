#![feature(slice_patterns)]
// Top-level todos:
// - add CLI options
// - add callbacks/hooks to run scripts (package manager, nvim update, etc)
// - automatic host detection; make it easy to set up new host based on platform templates
// - coordinate updating multiple templates in one command
// - separate config/template repository; maintain local checkout from git
//

mod config;
mod errors;
mod module;
mod repo_config;
mod template;
mod util;

use crate::module::Module;

#[macro_use]
extern crate clap;

fn main() {
    let matches = clap_app!(zotfile =>
      (version: "0.1")
      (author: "Zach Kemp <zvkemp@gmail.com>")
      (about: "Multi-target config manager")
      (@arg TARGET: -t --target +takes_value "target config toml file")
      (@arg MODULE: -m --module +takes_value "module to process")
    )
    .get_matches();

    let args = matches.args;
    let module = args
        .get("MODULE")
        .expect("please supply a module")
        .vals
        .get(0)
        .unwrap();
    let target = args
        .get("TARGET")
        .expect("please supply a target")
        .vals
        .get(0)
        .unwrap();
    let target_config = config::load_target_config(target.to_str().unwrap()).unwrap();
    let module =
        Module::new(module.to_str().unwrap(), target_config).expect("couldn't load module config");

    dbg!(module.process()).unwrap();
}
