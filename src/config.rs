use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub user: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_mode")]
    pub mode: String,
    #[serde(default = "default_base")]
    pub base: String,
    pub database: DatabaseConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_name")]
    pub name: String,
    #[serde(default = "default_ssl")]
    pub ssl: bool,
    #[serde(default = "default_sa")]
    pub sa: User,
    #[serde(default = "default_api")]
    pub api: User,
}

fn default_host() -> String { "localhost".to_string() }
fn default_port() -> u16 { 5432 }
fn default_name() -> String { "postgres".to_string() }
fn default_ssl() -> bool { false }
fn default_sa() -> User { User { user: "postgres".to_string(), password: "postgres".to_string() } }
fn default_api() -> User { User { user: "api".to_string(), password: "0000".to_string() } }
fn default_mode() -> String { "dev".to_string() }
fn default_base() -> String { "sql".to_string() }

impl Default for Config {
    fn default() -> Self {
        Config {
            mode: default_mode(),
            base: default_base(),
            database: DatabaseConfig {
                host: default_host(),
                port: default_port(),
                name: default_name(),
                ssl: default_ssl(),
                sa: default_sa(),
                api: default_api(),
            },
        }
    }
}

impl Config {
    /// Load configuration from TOML file and override with environment variables
    pub fn load(config_path: Option<&str>) -> Result<Self, Box<dyn std::error::Error>> {
        let mut config = Self::default();

        // Try to load from config file if provided or if db.toml exists
        let config_file = config_path.unwrap_or("db.toml");
        if Path::new(config_file).exists() {
            let content = fs::read_to_string(config_file)?;
            let file_config: Config = toml::from_str(&content)?;
            config = file_config;
        }

        // Override with environment variables
        config.apply_env_overrides();

        Ok(config)
    }

    /// Override configuration values with environment variables
    fn apply_env_overrides(&mut self) {
        if let Ok(value) = std::env::var("DB_HOST") {
            self.database.host = value;
        }
        if let Ok(value) = std::env::var("DB_PORT") {
            if let Ok(port) = value.parse() {
                self.database.port = port;
            }
        }
        if let Ok(value) = std::env::var("DB_SA_USER") {
            self.database.sa.user = value;
        }
        if let Ok(value) = std::env::var("DB_SA_PASSWORD") {
            self.database.sa.password = value;
        }
        if let Ok(value) = std::env::var("DB_API_USER") {
            self.database.api.user = value;
        }
        if let Ok(value) = std::env::var("DB_API_PASSWORD") {
            self.database.api.password = value;
        }
        if let Ok(value) = std::env::var("DB_NAME") {
            self.database.name = value;
        }
        if let Ok(value) = std::env::var("DB_SSL") {
            if let Ok(ssl) = value.parse() {
                self.database.ssl = ssl;
            }
        }
        if let Ok(value) = std::env::var("DB_MODE") {
            self.mode = value;
        }
        if let Ok(value) = std::env::var("DB_BASE") {
            self.base = value;
        }
        // Also support DATABASE_URL for convenience
        if let Ok(url) = std::env::var("DATABASE_URL") {
            // Parse DATABASE_URL format: postgres://user:password@host:port/database
            if let Ok(parsed) = Self::parse_database_url(&url) {
                self.database = parsed;
            }
        }
    }

    /// Parse a DATABASE_URL into DatabaseConfig
    fn parse_database_url(url: &str) -> Result<DatabaseConfig, Box<dyn std::error::Error>> {
        // Simple parser for postgres://user:password@host:port/database
        let url = url.strip_prefix("postgres://").unwrap_or(url);

        let (auth, rest) = url.split_once('@').ok_or("Invalid DATABASE_URL format")?;
        let (user, password) = auth.split_once(':').unwrap_or((auth, ""));

        let (host_port, name) = rest.split_once('/').ok_or("Invalid DATABASE_URL format")?;
        let (host, port_str) = host_port.split_once(':').unwrap_or((host_port, "5432"));
        let port = port_str.parse().unwrap_or(5432);

        Ok(DatabaseConfig {
            host: host.to_string(),
            port,
            name: name.to_string(),
            ssl: false,
            sa: User { user: user.to_string(), password: password.to_string() },
            api: default_api(),
        })
    }

    /// Get the database connection string
    pub fn database_url(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.database.sa.user,
            self.database.sa.password,
            self.database.host,
            self.database.port,
            self.database.name
        )
    }

    pub fn sql_base(&self) -> &str {
        &self.base
    }

    /// Check if in development mode
    pub fn is_dev(&self) -> bool {
        self.mode == "dev" || self.mode == "development"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_database_url() {
        let url = "postgres://user:pass@localhost/mydb";
        let config = Config::parse_database_url(url).unwrap();
        assert_eq!(config.sa.user, "user");
        assert_eq!(config.sa.password, "pass");
        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 5432);
        assert_eq!(config.name, "mydb");
    }
}
