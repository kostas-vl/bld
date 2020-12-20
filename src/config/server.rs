use crate::types::{BldError, Result};
use yaml_rust::Yaml;

#[derive(Debug)]
pub struct BldServerConfig {
    pub name: String,
    pub host: String,
    pub port: i64,
}

impl BldServerConfig {
    pub fn load(yaml: &Yaml) -> Result<Self> {
        let name = match yaml["server"].as_str() {
            Some(name) => name.to_string(),
            None => {
                let message = "Server entry must have a name".to_string();
                return Err(BldError::Other(message));
            }
        };

        let host = match yaml["host"].as_str() {
            Some(host) => host.to_string(),
            None => {
                let message = "Server entry must have a host address".to_string();
                return Err(BldError::Other(message));
            }
        };

        let port = match yaml["port"].as_i64() {
            Some(port) => port,
            None => {
                let message = "Server entry must define a port".to_string();
                return Err(BldError::Other(message));
            }
        };

        Ok(Self { name, host, port })
    }
}
