// Top-level todos:
// - add an interaction layer (Cursive or termion) to show
//   diffs and interactively update the templates or local files
// - build diff of template and existing file
// - add CLI options
// - add host configs
// - add template configs
// - add callbacks/hooks to run scripts (package manager, nvim update, etc)
// - automatic host detection; make it easy to set up new host based on platform templates
// - coordinate updating multiple templates in one command
// - separate config/template repository; maintain local checkout from git
//
fn main() {
    let mut template = Template::new_from_file(
            "templates/test/hello.hbs",
            "test/test.conf",
            HostConfig { username: String::from("zach") },
            None,
        );

    println!("{}", template.render());
}

use std::path::Path;
use handlebars::to_json;
use toml;

type Custom = Option<toml::Value>;

#[derive(Debug)]
pub struct Template {
    host_config: HostConfig,
    template_string: String,
    target_path: String,
    custom: Custom,
}

// TODO:
// read from toml
#[derive(Debug)]
pub struct HostConfig {
    username: String,
}

mod util {
    pub fn whoami<'a>() -> String {
        let stdout = std::process::Command::new("whoami").output().expect("tried to get username").stdout;
        std::str::from_utf8(&stdout).expect("").trim().into()
    }
}

impl HostConfig {
    pub fn default() -> Self {
        HostConfig { username: util::whoami() }
    }
}


impl Template {
    pub fn new_from_file(template_path: &str, target_path: &str, host_config: HostConfig, custom: Custom) -> Self {
        fn read_template(path: &str) -> std::io::Result<String> {
            use std::fs::File;
            use std::io::prelude::*;
            use std::io::BufReader;

            let file = File::open(path)?;
            let mut buf_reader = BufReader::new(file);
            let mut contents = String::new();
            buf_reader.read_to_string(&mut contents)?;
            Ok(contents)
        }

        Self::new(&read_template(template_path).unwrap(), target_path, host_config, custom)
    }

    pub fn new(template_string: &str, target_path: &str, host_config: HostConfig, custom: Custom) -> Self {
        Template {
            host_config,
            target_path: String::from(target_path),
            template_string: String::from(template_string),
            custom
        }
    }

    pub fn render(&self) -> String {
        use handlebars::Handlebars;
        let reg = Handlebars::new();
        println!("{}", to_json(&self));
        reg.render_template(&self.template_string, &to_json(&self)).unwrap()
    }
}

use serde::ser::{Serialize, SerializeStruct, Serializer};

impl Serialize for HostConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_struct("HostConfig", 1)?;
        s.serialize_field("username", &self.username)?;
        s.end()
    }
}

impl Serialize for Template {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // TODO: add 'platform' to top level
        let mut s = serializer.serialize_struct("Template", 3)?;
        s.serialize_field("host_config", &self.host_config)?;
        s.serialize_field("target_path", &self.target_path)?;
        s.serialize_field("custom", &self.custom)?;
        s.end()
    }
}

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

        assert_eq!(tmpl.render(), format!("hello {}\n", util::whoami()));
    }

    #[test]
    fn test_rendering_template_with_custom_values() {
        let custom: Custom = "[alternate_greeting]\nen = \"greetings!\"".parse::<toml::Value>().ok();
        let tmpl = Template::new_from_file(
            "templates/test/hello.hbs",
            "target/test_out/hello.conf",
            HostConfig::default(),
            custom
        );

        assert_eq!(tmpl.render(), format!("hello {}\ngreetings!\n", util::whoami()));
    }
}
