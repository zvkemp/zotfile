fn main() {
    let mut template = Template::new(
            "templates/test.handlebars",
            "test/test.conf",
            HostConfig { username: String::from("zach") }
        );


    println!("{}", template.render());
}

use std::path::Path;
use handlebars::to_json;

#[derive(Debug)]
pub struct Template {
    host_config: HostConfig,
    template_string: String,
    target_path: String,
}

// TODO:
// read from toml
#[derive(Debug)]
pub struct HostConfig {
    username: String,
}

impl Template {
    pub fn new(template_path: &str, target_path: &str, host_config: HostConfig) -> Self {
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

        Template {
            host_config,
            target_path: String::from(target_path),
            template_string: read_template(template_path).unwrap(),
        }
    }

    pub fn render(&self) -> String {
        use handlebars::Handlebars;
        let mut reg = Handlebars::new();
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
        let mut s = serializer.serialize_struct("Template", 2)?;
        s.serialize_field("host_config", &self.host_config)?;
        s.serialize_field("target_path", &self.target_path)?;
        s.end()
    }
}
