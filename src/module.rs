use ansi_term::Colour;
use std::error::Error;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;

use serde::Deserialize;

use crate::config::{Config, HostConfig};
use crate::repo_config::RepoConfig;
use crate::template::Template;
use crate::errors;

pub struct Module<'a> {
    name: &'a str,
    target_config: Config,
    host_config: HostConfig,
    module_config: Config,
}

#[derive(Debug, Deserialize)]
struct AfterCommitHook {
    shell: Option<String>
}

use std::process::Stdio;
use std::process::Command;

impl AfterCommitHook {
    pub fn process(&self) -> errors::Result<()> {
        match self.shell {
            Some(ref command) => {
                println!("{}", Colour::Green.paint(format!("Running `{}`", command)));
                match command.split(' ').collect::<Vec<&str>>().as_slice() {
                    [cmd, args..] => {
                        let mut p = Command::new(cmd)
                            .stdin(Stdio::piped())
                            .stderr(Stdio::piped())
                            .stdout(Stdio::piped())
                            .args(args)
                            .spawn()?;

                        p.wait()?;
                    },
                    _ => ()
                }
            },

            _ => ()
        };
        Ok(())
    }
}

impl<'a> Module<'a> {
    pub fn new(name: &'a str, target_config: Config) -> errors::Result<Self> {
        let host_config = HostConfig::default();
        let mut module = Module { host_config, name, target_config, module_config: None };
        module.maybe_load_module_config()?;
        Ok(module)
    }

    fn maybe_load_module_config(&mut self) -> errors::Result<()> {
        let conf_path = format!("modules/{}/config.toml", self.name);
        let path = Path::new(&conf_path);
        if !path.is_file() { return Ok(()); }

        let template = Template::new_from_file(
            &conf_path,
            &self.host_config,
            &self.target_config,
            &None,
        ).render();

        let module_config = template.parse::<toml::Value>()?;

        self.module_config = Some(module_config);
        Ok(())
    }

    pub fn process(&self) -> errors::Result<()> {
        self.process_repos()?;
        self.process_templates()?;
        self.process_after_commits()?;

        Ok(())
    }

    fn process_repos(&self) -> Result<(), std::io::Error> {
        match self.module_config {
            Some(ref toml) => {
                match &toml.get("repos") {
                    Some(toml::Value::Array(v)) => {
                        for r in v.iter() {
                            let repo = r.clone().try_into::<RepoConfig>().expect("hmm");
                            repo.go_do().unwrap();
                        }
                    },

                    _ => { println!("No repos to clone, skipping"); }
                }
                // for repo in (&toml["repos"]).iter() {
                //     dbg!(repo);
                // }
            },
            _ => { panic!("bar"); }
        }

        Ok(())
    }

    fn process_after_commits(&self) -> Result<(), std::io::Error> {
        match self.module_config {
            Some(ref toml) => {
                match &toml.get("after_commit") {
                    Some(toml::Value::Array(v)) => {
                        for r in v.iter() {
                            let hook = r.clone().try_into::<AfterCommitHook>().expect("hmm");
                            hook.process();
                        }
                    },
                    _ => {}
                }
            },

            _ => {}
        };

        Ok(())
    }

    fn process_templates(&self) -> Result<(), std::io::Error> {
        for path in self.template_paths()? {
            let template = Template::new_from_file(
                    path.unwrap().path().to_str().expect(""),
                    &self.host_config,
                    &self.target_config,
                    &None,
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
                                mkdir_p(&path);
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

fn mkdir_p(path: &Path) -> () {
    dbg!(path.parent().map(|x| {
        std::fs::create_dir_all(x)
    }));
}
