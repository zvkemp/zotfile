use handlebars::{Handlebars, to_json};
use serde::ser::{Serialize, Serializer, SerializeStruct};
use std::collections::HashMap;
use std::io::{prelude::*};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::config::{Config, HostConfig};
use crate::util;
use crate::errors;

#[derive(Debug)]
pub struct Template<'a> {
    host_config: &'a HostConfig,
    template_string: String,
    target_config: &'a Config, // machine-specific config
    module_config: &'a Config, // module-specific config, e.g., modules/<mod>/config.toml
    template_config: Config    // template-specific variables
}

impl<'a> Template<'a> {
    pub fn new_from_file(
        template_path: &str,
        host_config: &'a HostConfig,
        target_config: &'a Config,
        module_config: &'a Config) -> errors::Result<Self> {

        fn read_template(path: &str) -> errors::Result<(String, String)> {
            let raw_contents = util::read_file_to_string(&Path::new(path))?;
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

        let (template, template_config_raw) = read_template(template_path)?;

        // initial render of frontmatter only
        let template_config = Self::new(&template_config_raw,
                                        &host_config,
                                        &target_config,
                                        None,
                                        &None).render().parse().ok();

        Ok(Self::new(&template, host_config, target_config, template_config, module_config))
    }

    pub fn new(template_string: &str,
               host_config: &'a HostConfig,
               target_config: &'a Config,
               template_config: Config,
               module_config: &'a Config) -> Self {
        Template {
            host_config,
            template_string: String::from(template_string),
            target_config,
            module_config,
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
        let parts = vec![
            "!!!!!!!!!!",
            "Warning!",
            "This file was generated by Zotfile, an automated config manager.",
            "Changes may be overwritten from time to time.",
            "!!!!!!!!!!",
        ];

        let mut result = String::new();

        let comment_formatter = match self.template_config {
            Some(ref conf) => {
                match conf.get("comment_format") {
                    Some(toml) => toml.as_str(),
                    _ => None
                }
            },
            _ => None
        }.unwrap_or("# ");

        for part in parts {
            result.push_str(&format!("{}{}\n", comment_formatter, part));
        }

        result
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
        if let Some(conf) = &self.target_config {
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
            // None
            panic!("no...");
        }
    }


    pub fn dirs(&self) -> HashMap<&str, PathBuf> {
        let mut h = HashMap::new();
        dirs::home_dir().map(|d| {
            h.insert("home", d.clone());
            // NOTE: dirs::config_dir() points to ~/Library/Preferences, which is not what we want
            // in most cases.
            h.insert("config", d.join(Path::new(".config")));
        });

        h
    }
}


impl<'a> Serialize for Template<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // TODO: add 'platform' to top level
        let mut s = serializer.serialize_struct("Template", 3)?;
        s.serialize_field("host", &self.host_config)?;
        s.serialize_field("target", &self.target_config)?;
        s.serialize_field("module", &self.module_config)?;
        s.serialize_field("dirs", &self.dirs())?;
        s.serialize_field("copy_command", &self.copy_command())?; // FIXME should this live under the target config?
        s.end()
    }
}

#[cfg(test)]
mod test {

    #[test]
    fn test_sanity() {
        assert!(true);
    }
}
