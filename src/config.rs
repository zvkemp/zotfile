use serde::ser::{Serialize, Serializer, SerializeStruct};
use toml;

use crate::util;

pub type Config = Option<toml::Value>;
pub type Distro = Option<String>;

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

