// Top-level todos:
// - add CLI options
// - add callbacks/hooks to run scripts (package manager, nvim update, etc)
// - automatic host detection; make it easy to set up new host based on platform templates
// - coordinate updating multiple templates in one command
// - separate config/template repository; maintain local checkout from git
//

use std::fs::File;
use std::io::{BufReader, prelude::*};
use std::path::Path;

mod repo_config;
mod module;
mod config;
mod util;
mod template;

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
    ).get_matches();

    let args = matches.args;
    let module = args.get("MODULE").expect("please supply a module").vals.get(0).unwrap();
    let target = args.get("TARGET").expect("please supply a target").vals.get(0).unwrap();

    let target_config = {
        let path = Path::new(target);
        let file = File::open(path).unwrap();
        let mut buf_reader = BufReader::new(file);
        let mut contents = String::new();
        buf_reader.read_to_string(&mut contents).unwrap();
        contents.parse::<toml::Value>().ok()
    };

    let module = Module::new(module.to_str().unwrap(), target_config);

    module.process_repos().unwrap();
    module.process_templates().unwrap();
}

//  FIXME: these tests need to be updated
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_rendering_template() {
        let tmpl = Template::new_from_file(
            "templates/test/hello.hbs",
            "target/test_out/hello.conf",
            HostConfig::default(),
            None,
        );

        assert_eq!(tmpl.render(), format!("hello {}\nHello!\n", util::whoami()));
    }

    #[test]
    fn test_rendering_template_with_custom_values() {
        let custom: Config = "[alternate_greeting]\nen = \"greetings!\"".parse::<toml::Value>().ok();
        let tmpl = Template::new_from_file(
            "templates/test/hello.hbs",
            "target/test_out/hello.conf",
            HostConfig::default(),
            custom
        );

        tmpl.diff();

        assert_eq!(tmpl.render(), format!("hello {}\ngreetings!\n", util::whoami()));
    }
}
