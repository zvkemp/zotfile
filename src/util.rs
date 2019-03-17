use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::Path;

use crate::config::{Config, Platform};
use crate::errors;

pub fn whoami<'a>() -> String {
    let stdout = std::process::Command::new("whoami")
        .output()
        .expect("tried to get username")
        .stdout;
    std::str::from_utf8(&stdout).expect("").trim().into()
}

pub fn hostname() -> String {
    let stdout = std::process::Command::new("uname")
        .args(&["-n"])
        .output()
        .expect("tried to get username")
        .stdout;
    std::str::from_utf8(&stdout).expect("").trim().into()
}

pub fn platform() -> Platform {
    let stdout = std::process::Command::new("uname")
        .args(&["-o"])
        .output()
        .expect("tried to get username")
        .stdout;
    let p = std::str::from_utf8(&stdout).expect("").trim();
    match p {
        "GNU/Linux" => Platform::Linux(None),
        _ => Platform::Unknown,
    }
}

pub fn read_file_to_string(path: &Path) -> errors::Result<String> {
    let file = File::open(path).expect(&format!("file {:?} not found", path));
    let mut buf_reader = BufReader::new(file);
    let mut contents = String::new();
    buf_reader.read_to_string(&mut contents)?;
    Ok(contents)
}

pub fn load_toml_file(path: &Path) -> errors::Result<Config> {
    let result = read_file_to_string(path)?.parse::<toml::Value>().ok();
    Ok(result)
}
