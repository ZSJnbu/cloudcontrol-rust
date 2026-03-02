use cloudcontrol::config::AppConfig;

#[test]
fn test_load_real_config() {
    let config = AppConfig::load("config/default_dev.yaml");
    assert!(config.is_ok(), "Should load real config: {:?}", config.err());
}

#[test]
fn test_config_server_port() {
    let config = AppConfig::load("config/default_dev.yaml").unwrap();
    assert!(config.server.port > 0, "Port should be positive");
    assert!(config.server.port < 65535, "Port should be valid");
}

#[test]
fn test_config_db_name() {
    let config = AppConfig::load("config/default_dev.yaml").unwrap();
    assert!(
        !config.db_configs.db_name.is_empty(),
        "DB name should not be empty"
    );
    assert!(
        config.db_configs.db_name.ends_with(".db"),
        "DB name should end with .db"
    );
}
