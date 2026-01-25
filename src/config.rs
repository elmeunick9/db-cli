use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::collections::HashMap;

fn default_host() -> String { "localhost".to_string() }
fn default_port() -> u16 { 5432 }
fn default_name() -> String { "postgres".to_string() }
fn default_ssl() -> bool { false }
fn default_sa() -> User { User { user: "postgres".to_string(), password: "postgres".to_string() } }
fn default_api() -> User { User { user: "api".to_string(), password: "0000".to_string() } }
fn default_maintenance_db_name() -> String { "postgres".to_string() }
fn default_mode() -> String { "dev".to_string() }
fn default_base() -> String { "sql".to_string() }
fn default_auto_set_search_path() -> bool { true }
fn default_dry_run() -> bool { false }
fn default_log_sql() -> bool { true }
fn default_keep_max_releases() -> usize { 5 }
fn default_ai_enabled() -> bool { true }
fn default_ai_provider() -> String { "OpenRouter".to_string() }
fn default_ai_model() -> String { "arcee-ai/trinity-mini:free".to_string() }
fn default_ai_api_key() -> String { "".to_string() }
fn default_sql_dialect() -> String { "postgres".to_string() }
fn default_log_secrets() -> bool { false }
fn default_sql_base() -> SqlBase { SqlBase::Single("sql".to_string()) }
fn default_working_db() -> Vec<String> { vec![] }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub user: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    #[serde(default = "default_ai_enabled")]
    pub enabled: bool,
    #[serde(default = "default_ai_provider")]
    pub provider: String,
    #[serde(default = "default_ai_model")]
    pub model: String,
    #[serde(default = "default_ai_api_key")]
    pub api_key: String,
}

impl Default for AiConfig {
    fn default() -> Self {
        AiConfig {
            enabled: default_ai_enabled(),
            provider: default_ai_provider(),
            model: default_ai_model(),
            api_key: default_ai_api_key(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ReferenceValue {
    String(String),
    StringArray(Vec<String>),
    NestedArray(Vec<ReferenceValue>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SqlBase {
    Single(String),
    Multiple(Vec<String>),
}

impl SqlBase {
    pub fn to_vec(&self) -> Vec<String> {
        match self {
            SqlBase::Single(s) => vec![s.clone()],
            SqlBase::Multiple(v) => v.clone(),
        }
    }
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
    #[serde(default = "default_maintenance_db_name")]
    pub maintenance_db_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_mode")]
    pub mode: String,
    #[serde(default = "default_sql_base")]
    pub base: SqlBase,
    #[serde(default = "default_sql_dialect")]
    pub sql_dialect: String,
    #[serde(default = "default_working_db")]
    pub working_db: Vec<String>,
    #[serde(default = "default_auto_set_search_path")]
    pub auto_set_search_path: bool,
    #[serde(default = "default_dry_run")]
    pub dry_run: bool,
    #[serde(default = "default_keep_max_releases")]
    pub keep_max_releases: usize,
    #[serde(default = "default_log_sql")]
    pub log_sql: bool,
    #[serde(default = "default_log_secrets")]
    pub log_secrets: bool,
    pub database: DatabaseConfig,
    #[serde(default)]
    pub ai: AiConfig,
    #[serde(default)]
    pub references: HashMap<String, ReferenceValue>,
    #[serde(default)]
    pub secrets: HashMap<String, String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            mode: default_mode(),
            base: default_sql_base(),
            sql_dialect: default_sql_dialect(),
            working_db: default_working_db(),
            auto_set_search_path: default_auto_set_search_path(),
            dry_run: default_dry_run(),
            keep_max_releases: default_keep_max_releases(),
            log_sql: default_log_sql(),
            log_secrets: default_log_secrets(),
            database: DatabaseConfig {
                host: default_host(),
                port: default_port(),
                name: default_name(),
                maintenance_db_name: default_maintenance_db_name(),
                ssl: default_ssl(),
                sa: default_sa(),
                api: default_api(),
            },
            ai: AiConfig::default(),
            references: HashMap::new(),
            secrets: HashMap::new(),
        }
    }
}

impl Config {
    /// Validate that a key is in proper snake_case format (lowercase, alphanumeric, underscores only)
    fn validate_key_format(key: &str) -> Result<(), String> {
        if key.is_empty() {
            return Err("Key cannot be empty".to_string());
        }
        
        // Check if it matches snake_case pattern: lowercase, digits, underscores
        // Must start with lowercase letter, can contain underscores and digits
        if !key.chars().next().unwrap().is_lowercase() {
            return Err(format!("Key '{}' must start with a lowercase letter", key));
        }
        
        for c in key.chars() {
            if !c.is_ascii_lowercase() && !c.is_ascii_digit() && c != '_' {
                return Err(format!(
                    "Key '{}' contains invalid character '{}'. Keys must be in snake_case: lowercase letters, digits, and underscores only",
                    key, c
                ));
            }
        }
        
        if key.ends_with('_') || key.starts_with('_') {
            return Err(format!("Key '{}' cannot start or end with underscore", key));
        }
        
        if key.contains("__") {
            return Err(format!("Key '{}' cannot contain consecutive underscores", key));
        }
        
        Ok(())
    }

    /// Validate that a SQL dialect is supported
    fn validate_dialect(dialect: &str) -> Result<(), String> {
        match dialect {
            "postgres" | "postgresql" | "mysql" | "mssql" | "sqlite" => Ok(()),
            _ => Err(format!(
                "Unsupported SQL dialect '{}'. Supported dialects: postgres, postgresql, mysql, mssql, sqlite",
                dialect
            )),
        }
    }

    /// Load configuration from TOML file and override with environment variables
    pub fn load(config_path: Option<&str>) -> Result<Self, Box<dyn std::error::Error>> {
        let mut config = Self::default();

        // Try to load from config file if provided or if db.toml exists
        let config_file = config_path.unwrap_or("db.toml");
        if Path::new(config_file).exists() {
            let content = fs::read_to_string(config_file)?;
            let file_config: Config = toml::from_str(&content)?;
            
            // Validate reference keys
            for key in file_config.references.keys() {
                Self::validate_key_format(key)?;
            }

            // Validate secret keys
            for key in file_config.secrets.keys() {
                Self::validate_key_format(key)?;
            }
            
            // Validate SQL dialect early
            Self::validate_dialect(&file_config.sql_dialect)?;
            
            config = file_config;
        }

        // De-obfuscate public/free ApiKey for OpenRouter (temporal fix)
        config.ai.api_key = config.ai.api_key.replace("free:", "sk-or-").replace("&", "e");

        // Override with environment variables
        config.apply_env_overrides();

        Ok(config)
    }

    /// Load and merge configuration from a specific path, with hierarchical overrides
    /// Returns config with single sql_base entry from that directory
    pub fn load_for_path(
        base_path: &str,
        root_config_path: Option<&str>
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // Load root config first
        let mut merged = Self::load(root_config_path)?;

        // Look for db.toml in the base_path directory
        let local_config_path = Path::new(base_path).join("db.toml");
        if local_config_path.exists() {
            let content = fs::read_to_string(&local_config_path)?;
            
            // Parse as table to check which keys are present
            let table: toml::Table = toml::from_str(&content)?;
            
            // Parse as config to get the values
            let local_config: Config = toml::from_str(&content)?;
            
            // Validate SQL dialect
            Self::validate_dialect(&local_config.sql_dialect)?;
            
            // Only override fields that exist in the local config
            if table.contains_key("mode") { merged.mode = local_config.mode; }
            if table.contains_key("base") { merged.base = local_config.base; }
            if table.contains_key("sql_dialect") { merged.sql_dialect = local_config.sql_dialect; }
            if table.contains_key("working_db") { merged.working_db = local_config.working_db; }
            if table.contains_key("auto_set_search_path") { merged.auto_set_search_path = local_config.auto_set_search_path; }
            if table.contains_key("dry_run") { merged.dry_run = local_config.dry_run; }
            if table.contains_key("keep_max_releases") { merged.keep_max_releases = local_config.keep_max_releases; }
            if table.contains_key("log_sql") { merged.log_sql = local_config.log_sql; }
            if table.contains_key("log_secrets") { merged.log_secrets = local_config.log_secrets; }
            if table.contains_key("database") { merged.database = local_config.database; }
            if table.contains_key("ai") { merged.ai = local_config.ai; }
            if table.contains_key("references") { merged.references = local_config.references; }
            if table.contains_key("secrets") { merged.secrets = local_config.secrets; }
        }

        // Ensure sql_base is a single string for this path
        merged.base = SqlBase::Single(base_path.to_string());
        Ok(merged)
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
            self.base = SqlBase::Single(value);
        }
        if let Ok(value) = std::env::var("DB_LOG_SQL") {
            if let Ok(log_sql) = value.parse() {
                self.log_sql = log_sql;
            }
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
            maintenance_db_name: default_maintenance_db_name(),
            ssl: default_ssl(),
            sa: User { user: user.to_string(), password: password.to_string() },
            api: default_api(),
        })
    }

    /// Get the database connection string based on configured dialect
    pub fn database_url(&self) -> String {
        let scheme = match self.sql_dialect.as_str() {
            "postgres" | "postgresql" => "postgres",
            "mysql" => "mysql",
            "mssql" => "mssql",
            _ => "postgres",
        };
        
        format!(
            "{}://{}:{}@{}:{}/{}",
            scheme,
            self.database.sa.user,
            self.database.sa.password,
            self.database.host,
            self.database.port,
            self.database.name
        )
    }

    pub fn sql_base(&self) -> String {
        match &self.base {
            SqlBase::Single(s) => s.clone(),
            SqlBase::Multiple(v) => v.get(0).cloned().unwrap_or_else(|| "sql".to_string()),
        }
    }

    pub fn sql_base_list(&self) -> Vec<String> {
        self.base.to_vec()
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

    #[test]
    fn test_validate_key_format_valid() {
        assert!(Config::validate_key_format("meta_key").is_ok());
        assert!(Config::validate_key_format("my_table_ref").is_ok());
        assert!(Config::validate_key_format("ref123").is_ok());
        assert!(Config::validate_key_format("a").is_ok());
    }

    #[test]
    fn test_validate_key_format_uppercase() {
        let result = Config::validate_key_format("MetaKey");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("lowercase letter"));
    }

    #[test]
    fn test_validate_key_format_special_chars() {
        let result = Config::validate_key_format("meta-key");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid character"));
    }

    #[test]
    fn test_validate_key_format_leading_underscore() {
        let result = Config::validate_key_format("_meta_key");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("start or end with underscore"));
    }

    #[test]
    fn test_validate_key_format_trailing_underscore() {
        let result = Config::validate_key_format("meta_key_");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("start or end with underscore"));
    }

    #[test]
    fn test_validate_key_format_consecutive_underscores() {
        let result = Config::validate_key_format("meta__key");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("consecutive underscores"));
    }

    #[test]
    fn test_validate_key_format_empty() {
        let result = Config::validate_key_format("");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty"));
    }
}
