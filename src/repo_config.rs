use serde::{Serialize, Deserialize};
use git2::Repository;
use git2::{ErrorCode, ErrorClass};
use ansi_term::Colour;

#[derive(Debug, Serialize, Deserialize)]
pub struct RepoConfig {
    path: String,
    url: String,
    sha: Option<String>,
}

impl RepoConfig {
    pub fn go_do(&self) -> Result<(), git2::Error> {
        dbg!(self);

        let repo = match Repository::open(&self.path) {
            Ok(repo) => {
                repo.find_remote("origin")?.fetch(&["master"], None, None)?;
                repo
            },
            Err(e) => {
                match e.code() {
                    ErrorCode::NotFound => {
                        println!("{} {}",
                                 Colour::Green.bold().paint("Cloning"),
                                 Colour::Cyan.bold().paint(&self.url));

                        match Repository::clone(&self.url, &self.path) {
                            Ok(repo) => repo,
                            Err(e) => panic!(e)
                        }
                    },
                    _ => panic!(e)
                }
            }
        };

        // FIXME: checkout sha if given
        Ok(())
    }

}
