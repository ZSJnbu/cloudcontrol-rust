use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct AppConfig {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub db_configs: DbConfig,
    #[serde(default)]
    pub descption: Option<String>,
    // Legacy configs kept for compatibility
    #[serde(default)]
    pub redis_configs: Option<serde_yaml::Value>,
    #[serde(default)]
    pub kafka_configs: Option<serde_yaml::Value>,
    #[serde(default)]
    pub rest_server_configs: Option<serde_yaml::Value>,
    #[serde(default)]
    pub influxdb_configs: Option<serde_yaml::Value>,
    #[serde(rename = "SPIDER", default)]
    pub spider: Option<serde_yaml::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_port")]
    pub port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self { port: default_port() }
    }
}

fn default_port() -> u16 {
    8000
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct DbConfig {
    #[serde(default = "default_db_type")]
    pub r#type: String,
    #[serde(default = "default_db_name")]
    pub db_name: String,
    #[serde(default)]
    pub user: Option<String>,
    #[serde(default)]
    pub passwd: Option<String>,
    #[serde(default)]
    pub db_name1: Option<String>,
}

impl Default for DbConfig {
    fn default() -> Self {
        Self {
            r#type: default_db_type(),
            db_name: default_db_name(),
            user: None,
            passwd: None,
            db_name1: None,
        }
    }
}

fn default_db_type() -> String {
    "sqlite".to_string()
}

fn default_db_name() -> String {
    "cloudcontrol.db".to_string()
}

impl AppConfig {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: AppConfig = serde_yaml::from_str(&content)?;
        tracing::info!("Configuration loaded successfully");
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_config_from_file() {
        let config = AppConfig::load("config/default_dev.yaml");
        assert!(config.is_ok(), "Should load config from file: {:?}", config.err());
        let config = config.unwrap();
        assert!(config.server.port > 0);
    }

    #[test]
    fn test_config_defaults() {
        let yaml = "{}";
        let config: AppConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.server.port, 8000);
        assert_eq!(config.db_configs.db_name, "cloudcontrol.db");
        assert_eq!(config.db_configs.r#type, "sqlite");
    }

    #[test]
    fn test_load_config_missing_file() {
        let result = AppConfig::load("nonexistent_path/config.yaml");
        assert!(result.is_err());
    }

    #[test]
    fn test_load_config_invalid_yaml() {
        let tmp = std::env::temp_dir().join("test_invalid.yaml");
        std::fs::write(&tmp, "{{{{invalid yaml!!!!").unwrap();
        let result = AppConfig::load(&tmp);
        assert!(result.is_err());
        let _ = std::fs::remove_file(&tmp);
    }
}
