// Top-level todos:
// - add CLI options
// - add callbacks/hooks to run scripts (package manager, nvim update, etc)
// - automatic host detection; make it easy to set up new host based on platform templates
// - coordinate updating multiple templates in one command
// - separate config/template repository; maintain local checkout from git
//

use ansi_term::Colour;
use handlebars::Handlebars;
use handlebars::to_json;
use std::error::Error;
use std::fs::File;
use std::io;
use std::io::BufReader;
use std::io::Write;
use std::io::prelude::*;
use std::path::Path;
use std::process::{Command, Stdio};
use toml;

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

    let custom_config = {
        let path = Path::new(target);
        let file = File::open(path).unwrap();
        let mut buf_reader = BufReader::new(file);
        let mut contents = String::new();
        buf_reader.read_to_string(&mut contents).unwrap();
        contents.parse::<toml::Value>().ok()
    };

    let module = Module::new(module.to_str().unwrap(), custom_config);

    module.process_templates().unwrap();
}

pub struct Module<'a> {
    name: &'a str,
    custom_config: Config
}

use std::fs;

impl<'a> Module<'a> {
    pub fn new(name: &'a str, custom_config: Config) -> Self {
        Module { name, custom_config }
    }

    pub fn process_templates(&self) -> Result<(), std::io::Error> {
        let host_config = HostConfig::default();
        for path in self.template_paths()? {
            let template = Template::new_from_file(
                    path.unwrap().path().to_str().expect(""),
                    &host_config,
                    &self.custom_config
                );

            let target_path = template.target_path().expect("target path exists");
            let mut less = std::process::Command::new("less");
            let mut child = less.stdin(std::process::Stdio::piped()).spawn().unwrap();

            // TODO:
            // commands (apply changes similar to git -p)?
            //
            // patch apply:
            //
            // stdin | git diff --no-index target/file/to/change - | patch -p1 target/file/to/change
            //
            // could intercept the second pipe and interactively stage individual hunks
            let diff = template.diff();

            let file_exists = Path::new(target_path).is_file();

            if diff.is_empty() && file_exists {
                println!("{} {}",
                         Colour::Green.bold().paint(target_path),
                         Colour::Cyan.bold().paint("is up to date."))
            } else {
                child.stdin.as_mut().map(|x| {
                    x.write_all(template.diff().as_bytes()).ok();
                });

                child.wait().unwrap();

                if file_exists {
                    println!("{} {} {}",
                             Colour::Yellow.paint("Apply changes?"),
                             Colour::Green.bold().paint(target_path),
                             Colour::Yellow.bold().paint("will be overwritten. [Y/n]"));
                } else {
                    println!("{} {} {}",
                             Colour::Yellow.bold().paint(target_path),
                             Colour::Green.bold().paint("does not yet exist. Proceed?"),
                             Colour::Yellow.bold().paint("[Y/n]"));
                }

                let mut input = String::new();
                match io::stdin().read_line(&mut input) {
                    Ok(_n) => {
                        match input.as_str().trim() {
                            "y" | "Y" => {
                                println!("{}", Colour::Yellow.paint(format!("saving `{}`...", &target_path)));

                                let path = Path::new(&target_path);
                                let mut file = match File::create(&path) {
                                    Err(e) => panic!("couldn't create {}: {}", path.display(), e.description()),
                                    Ok(file) => file
                                };

                                match file.write_all(template.render_with_warning().as_bytes()) {
                                    Err(e) => panic!("couldn't write {}: {}", path.display(), e.description()),
                                    Ok(_) => println!("{}", Colour::Green.paint("Done!")),
                                }
                            }
                            _ => ()
                        }
                    }

                    Err(n) => {
                        println!("{}", Colour::Red.paint(format!("error: {}", n)));
                    }
                }
            }
        }

        Ok(())
    }

    fn template_paths(&self) -> std::io::Result<fs::ReadDir> {
        fs::read_dir(Path::new(&format!("modules/{}/templates/", self.name)))
    }
}

type Config = Option<toml::Value>;

#[derive(Debug)]
pub struct Template<'a> {
    host_config: &'a HostConfig,
    template_string: String,
    custom: &'a Config, // machine-specific config
    template_config: Config // template-specific variables
}

type Distro = Option<String>;

#[derive(Clone, Debug)]
pub enum Platform {
    Linux(Distro),
    Darwin,
    Unknown
}

#[derive(Debug, Clone)]
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
        HostConfig {
            username: util::whoami(),
            hostname: util::hostname(),
            platform: util::platform(),
        }
    }
}

use std::collections::HashMap;
use std::path::PathBuf;

impl<'a> Template<'a> {
    pub fn new_from_file(template_path: &str, host_config: &'a HostConfig, custom: &'a Config) -> Self {
        fn read_template(path: &str) -> std::io::Result<(String, String)> {
            let file = File::open(path)?;
            let mut buf_reader = BufReader::new(file);
            let mut raw_contents = String::new();
            buf_reader.read_to_string(&mut raw_contents)?;

            let mut frontmatter = String::new();
            let mut contents = String::new();
            let mut in_frontmatter = false;

            for (i, line) in raw_contents.lines().enumerate() {
                match (i, line, in_frontmatter) {
                    (0, "---", false) => { in_frontmatter = true; },
                    (_, "---", true)  => { in_frontmatter = false; },
                    (_, line, true)   => { (&mut frontmatter).push_str(&format!("{}\n", line)); },
                    (_, line, false)  => { contents.push_str(&format!("{}\n", line)); }
                }
            }

            Ok((contents, frontmatter))
        }
        let (template, template_config_raw) = read_template(template_path).unwrap();

        // initial render of frontmatter only
        let template_config = Self::new(&template_config_raw,
                                        &host_config,
                                        &custom,
                                        None).render().parse().ok();
        Self::new(&template, host_config, custom, template_config)
    }

    pub fn new(template_string: &str,
               host_config: &'a HostConfig,
               custom: &'a Config,
               template_config: Config) -> Self {
        Template {
            host_config,
            template_string: String::from(template_string),
            custom,
            template_config
        }
    }

    pub fn render_with_warning(&self) -> String {
        let reg = Handlebars::new();
        let rendered = format!("{}\n{}", self.warning(), self.template_string);
        reg.render_template(&rendered, &to_json(&self)).unwrap()
    }

    pub fn render(&self) -> String {
        let reg = Handlebars::new();
        reg.render_template(&self.template_string, &to_json(&self)).unwrap()
    }

    pub fn target_path(&self) -> Option<&str> {
        let o = match self.template_config {
            Some(ref c) => c.get("target_path"),
            _ => None
        };

        o.expect("target_path should be in template frontmatter").as_str()
    }

    pub fn warning(&self) -> String {
        String::from("# !!!!!!!!!!!!\n# Warning! This file was generated by a script.\n# !!!!!!!!!!!!")
    }

    pub fn diff(&self) -> String {
        // FIXME stop unwrapping

        let mut p = Command::new("git")
            .stderr(Stdio::piped())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            // "color" option ensures ansi codes are rendered into the stdout pipe
            .args(&["diff", "--no-index", "--color", &self.target_path().clone().unwrap(), "-"])
            .spawn()
            .unwrap();

        p.stdin.as_mut().map(|x| x.write_all(self.render_with_warning().as_bytes()));

        let output = p.wait_with_output().unwrap();
        let result = String::from_utf8(output.stdout).unwrap();
        // println!("{}", &result);
        result
    }

    pub fn copy_command(&self) -> String {
        if let Some(conf) = &self.custom {
            match conf.get("clipboard") {
                Some(ref tv) => {
                    let s = tv.as_str().unwrap().clone();
                    match s {
                        "xclip" => String::from("xclip -i -selection clipboard"),
                        "xsel" => String::from("xsel -i --clipboard"),
                        "pbcopy" => String::from("pbcopy"),
                        s => panic!("clipboard {:?} not configured", s),
                    }
                }
                None => panic!("clipboard not configured")
            }
        } else {
            panic!("no...");
        }
    }


    pub fn dirs(&self) -> HashMap<&str, PathBuf> {
        let mut h = HashMap::new();
        dirs::home_dir().map(|d| h.insert("home", d));
        dirs::config_dir().map(|d| h.insert("config", d));
        h
    }
}

use serde::ser::{Serialize, SerializeStruct, Serializer};

impl Serialize for Platform {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str({
            match self {
                Platform::Linux(..) => "linux",
                Platform::Darwin => "macos",
                Platform::Unknown => "unknown",
            }
        })
    }
}


impl Serialize for HostConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_struct("HostConfig", 2)?;
        s.serialize_field("username", &self.username)?;
        s.serialize_field("platform", &self.platform)?;
        s.end()
    }
}

impl<'a> Serialize for Template<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // TODO: add 'platform' to top level
        let mut s = serializer.serialize_struct("Template", 3)?;
        s.serialize_field("host_config", &self.host_config)?;
        s.serialize_field("custom", &self.custom)?;
        s.serialize_field("dirs", &self.dirs())?;
        s.serialize_field("copy_command", &self.copy_command())?;
        s.end()
    }
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
