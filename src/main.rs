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
    let target_path = String::from("target/test_out/test.conf");
    let mut template = Template::new_from_file(
            "templates/test/hello.hbs",
            &target_path,
            HostConfig::default(),
            None,
        );

    println!("{}", template.render());

    let mut less = std::process::Command::new("less");
    let mut child = less.stdin(std::process::Stdio::piped()).spawn().unwrap();
    use std::io::Write;

    // TODO:
    // commands (apply changes similar to git -p)?
    //
    // patch apply:
    //
    // stdin | git diff --no-index target/file/to/change - | patch -p1 target/file/to/change
    //
    // could intercept the second pipe and interactively stage individual hunks
    child.stdin.as_mut().map(|x| {
        x.write_all(template.diff().as_bytes()).ok();
    });

    child.wait().unwrap();

    use ansi_term::Colour;
    use std::io;
    use std::fs::File;
    use std::path::Path;
    use std::error::Error;

    println!("Apply changes? {} will be overwritten. {}",
             Colour::Green.paint(&target_path),
             Colour::Yellow.paint("[Y/n]"));

    let mut input = String::new();
    match io::stdin().read_line(&mut input) {
        Ok(n) => {
            println!("{}", Colour::Green.paint(format!("Ok: {}", input)));
            match input.as_str().trim() {
                "y" | "Y" => {
                    println!("{}", Colour::Yellow.paint(format!("saving `{}`...", &target_path)));

                    let path = Path::new(&target_path);
                    let mut file = match File::create(&path) {
                        Err(e) => panic!("couldn't create {}: {}", path.display(), e.description()),
                        Ok(file) => file
                    };

                    match file.write_all(template.render().as_bytes()) {
                        Err(e) => panic!("couldn't write {}: {}", path.display(), e.description()),
                        Ok(_) => println!("{}", Colour::Green.paint("Done!")),
                    }
                }
                _ => println!("n")
            }
        }

        Err(n) => {
            println!("{}", Colour::Red.paint(format!("error: {}", n)));
        }
    }

}

use std::path::Path;
use handlebars::to_json;
use toml;

type Config = Option<toml::Value>;

#[derive(Debug)]
pub struct Template {
    host_config: HostConfig,
    template_string: String,
    target_path: String,
    custom: Config, // not sure what this will do yet
    template_config: Config // template-specific variables
}

type Distro = Option<String>;

#[derive(Debug)]
pub enum Platform {
    Linux(Distro),
    Darwin,
    Unknown
}

// TODO:
// read from toml
#[derive(Debug)]
pub struct HostConfig {
    username: String,
    hostname: String,
    platform: Platform,
}

mod util {
    use super::Platform;

    pub fn whoami<'a>() -> String {
        let stdout = std::process::Command::new("whoami").output().expect("tried to get username").stdout;
        std::str::from_utf8(&stdout).expect("").trim().into()
    }

    pub fn hostname() -> String {
        let stdout = std::process::Command::new("uname").args(&["-n"]).output().expect("tried to get username").stdout;
        std::str::from_utf8(&stdout).expect("").trim().into()
    }

    pub fn platform() -> Platform {
        let stdout = std::process::Command::new("uname").args(&["-o"]).output().expect("tried to get username").stdout;
        let p = std::str::from_utf8(&stdout).expect("").trim();
        match p {
            "GNU/Linux" => Platform::Linux(None),
            _ => Platform::Unknown
        }
    }
}

impl HostConfig {
    pub fn default() -> Self {
        dbg!(HostConfig {
            username: util::whoami(),
            hostname: util::hostname(),
            platform: util::platform(),
            // TODO: specific
        })
    }
}


impl Template {
    pub fn new_from_file(template_path: &str, target_path: &str, host_config: HostConfig, custom: Config) -> Self {
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

        Self::new(&read_template(template_path).unwrap(), target_path, host_config, custom, None)
    }

    pub fn new(template_string: &str,
               target_path: &str,
               host_config: HostConfig,
               custom: Config,
               template_config: Config) -> Self {
        Template {
            host_config,
            target_path: String::from(target_path),
            template_string: String::from(template_string),
            custom,
            template_config
        }
    }

    pub fn render(&self) -> String {
        use handlebars::Handlebars;
        let reg = Handlebars::new();
        // println!("{}", to_json(&self));
        reg.render_template(&self.template_string, &to_json(&self)).unwrap()
    }

    pub fn diff(&self) -> String {
        use std::io::Write;
        use std::process::{Command, Stdio};
        // FIXME stop unwrapping

        let mut p = Command::new("git")
            .stderr(Stdio::piped())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            // "color" option ensures ansi codes are rendered into the stdout pipe
            .args(&["diff", "--no-index", "--color", &self.target_path, "-"])
            .spawn()
            .unwrap();

        p.stdin.as_mut().map(|x| x.write_all(self.render().as_bytes()));

        let output = p.wait_with_output().unwrap();
        let result = String::from_utf8(output.stdout).unwrap();
        // println!("{}", &result);
        result
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
