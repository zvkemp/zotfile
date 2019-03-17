use std::path::Path;
use serde::ser::{Serialize, Serializer, SerializeStruct};
use toml;
use std::io::BufReader;
use std::io::prelude::*;
use std::fs::File;


use crate::util;
use crate::errors;

pub type Config = Option<toml::Value>;
pub type Distro = Option<String>;

pub fn load_target_config(name: &str) -> errors::Result<Config> {
    let config_path = format!("targets/{}.toml", name);
    let path = Path::new(&config_path);
    util::load_toml_file(&path)
}

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

impl HostConfig {
    pub fn default() -> Self {
        HostConfig {
            username: util::whoami(),
            hostname: util::hostname(),
            platform: util::platform(),
        }
    }
}

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


