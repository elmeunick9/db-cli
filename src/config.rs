use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use crate::utils::validators::{validate_key_format, validate_dialect};

fn default_host() -> String { "localhost".to_string() }
fn default_port() -> u16 { 5432 }
fn default_name() -> String { "postgres".to_string() }
fn default_ssl() -> bool { false }
fn default_sa() -> User { User { user: "postgres".to_string(), password: "postgres".to_string() } }
fn default_api() -> User { User { user: "api".to_string(), password: "0000".to_string() } }
fn default_maintenance_db_name() -> String { "postgres".to_string() }
fn default_mode() -> String { "dev".to_string() }
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
fn default_generate_entries() -> Vec<GenerateEntry> { vec![] }

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
pub struct GenerateEntry {
    pub format: String,
    #[serde(default)]
    pub input_dir: Option<String>,
    #[serde(default)]
    pub output_dir: Option<String>,
}

impl GenerateEntry {
    pub fn matches_format(&self, format_name: &str) -> bool {
        self.format.eq_ignore_ascii_case(format_name)
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
    #[serde(default = "default_generate_entries")]
    pub generate: Vec<GenerateEntry>,
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
            generate: default_generate_entries(),
            references: HashMap::new(),
            secrets: HashMap::new(),
        }
    }
}

impl Config {
    /// Load configuration from TOML file and override with environment variables
    pub fn load(path: Option<&str>) -> Result<(Self, toml::Table), Box<dyn std::error::Error>> {
        // Try to load from config file if provided or if db.toml exists
        let config_file = path.unwrap_or("db.toml");
        if !Path::new(config_file).exists() {
            panic!("Config path does not exist: {}", config_file);
        }

        let content = fs::read_to_string(config_file)?;
        let mut config: Config = toml::from_str(&content)?;
        let table = toml::from_str(&content)?;
            
        // Validate reference keys
        for key in config.references.keys() {
            validate_key_format(key)?;
        }

        // Validate secret keys
        for key in config.secrets.keys() {
            validate_key_format(key)?;
        }
        
        // Validate SQL dialect early
        validate_dialect(&config.sql_dialect)?;

        // De-obfuscate public/free ApiKey for OpenRouter (temporal fix)
        config.ai.api_key = config.ai.api_key.replace("free:", "sk-or-").replace("&", "e");

        // Override with environment variables
        config.apply_env_overrides();

        Ok((config, table))
    }

    /// Merge the provided config into self using a TOML table to detect which keys are present
    pub fn merge(mut self, config: Config, table: &toml::Table) -> Self {
        if table.contains_key("mode") { self.mode = config.mode; }
        if table.contains_key("base") { self.base = config.base; }
        if table.contains_key("sql_dialect") { self.sql_dialect = config.sql_dialect; }
        if table.contains_key("working_db") { self.working_db = config.working_db; }
        if table.contains_key("auto_set_search_path") { self.auto_set_search_path = config.auto_set_search_path; }
        if table.contains_key("dry_run") { self.dry_run = config.dry_run; }
        if table.contains_key("keep_max_releases") { self.keep_max_releases = config.keep_max_releases; }
        if table.contains_key("log_sql") { self.log_sql = config.log_sql; }
        if table.contains_key("log_secrets") { self.log_secrets = config.log_secrets; }

        if let Some(db_table) = table.get("database").and_then(|v| v.as_table()) {
            self.merge_database(config.database, db_table);
        }

        if let Some(ai_table) = table.get("ai").and_then(|v| v.as_table()) {
            self.merge_ai(config.ai, ai_table);
        }

        if let Some(ref_table) = table.get("references").and_then(|v| v.as_table()) {
            for key in ref_table.keys() {
                if let Some(value) = config.references.get(key) {
                    self.references.insert(key.clone(), value.clone());
                }
            }
        }

        if table.contains_key("generate") {
            self.generate = config.generate;
        }

        if let Some(secret_table) = table.get("secrets").and_then(|v| v.as_table()) {
            for key in secret_table.keys() {
                if let Some(value) = config.secrets.get(key) {
                    self.secrets.insert(key.clone(), value.clone());
                }
            }
        }

        self
    }

    /// Merge configuration from a single path into self using deep, key-aware merges
    pub fn merge_from(self, path: &str) -> Self {
        // If path is a directory, look for db.toml inside it
        let config_path = if Path::new(path).is_dir() {
            let db_toml_path = format!("{}/db.toml", path.trim_end_matches('/'));
            if Path::new(&db_toml_path).exists() {
                db_toml_path
            } else {
                // No db.toml in directory, return self without merging
                return self;
            }
        } else {
            path.to_string()
        };

        let (local_config, table) = Self::load(Some(&config_path))
            .unwrap_or_else(|e| panic!("Failed to load configuration from '{}': {}", config_path, e));
        self.merge(local_config, &table)
    }

    fn merge_database(&mut self, config: DatabaseConfig, table: &toml::Table) {
        if table.contains_key("host") { self.database.host = config.host; }
        if table.contains_key("port") { self.database.port = config.port; }
        if table.contains_key("name") { self.database.name = config.name; }
        if table.contains_key("ssl") { self.database.ssl = config.ssl; }
        if table.contains_key("sa") { self.database.sa = config.sa; }
        if table.contains_key("api") { self.database.api = config.api; }
        if table.contains_key("maintenance_db_name") { self.database.maintenance_db_name = config.maintenance_db_name; }
    }

    fn merge_ai(&mut self, config: AiConfig, table: &toml::Table) {
        if table.contains_key("enabled") { self.ai.enabled = config.enabled; }
        if table.contains_key("provider") { self.ai.provider = config.provider; }
        if table.contains_key("model") { self.ai.model = config.model; }
        if table.contains_key("api_key") { self.ai.api_key = config.api_key; }
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

    pub fn generate_output_dir(&self, format_name: &str) -> PathBuf {
        if let Some(entry) = self.generate.iter().find(|entry| entry.matches_format(format_name)) {
            if let Some(dir) = &entry.output_dir {
                return PathBuf::from(dir);
            }
        }

        PathBuf::from("gen").join(format_name)
    }

    pub fn generate_input_dir(&self, format_name: &str) -> Option<PathBuf> {
        self.generate
            .iter()
            .find(|entry| entry.matches_format(format_name))
            .and_then(|entry| entry.input_dir.as_ref())
            .map(PathBuf::from)
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
        assert!(validate_key_format("meta_key").is_ok());
        assert!(validate_key_format("my_table_ref").is_ok());
        assert!(validate_key_format("ref123").is_ok());
        assert!(validate_key_format("a").is_ok());
    }

    #[test]
    fn test_validate_key_format_uppercase() {
        let result = validate_key_format("MetaKey");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("lowercase letter"));
    }

    #[test]
    fn test_validate_key_format_special_chars() {
        let result = validate_key_format("meta-key");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid character"));
    }

    #[test]
    fn test_validate_key_format_leading_underscore() {
        let result = validate_key_format("_meta_key");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("start or end with underscore"));
    }

    #[test]
    fn test_validate_key_format_trailing_underscore() {
        let result = validate_key_format("meta_key_");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("start or end with underscore"));
    }

    #[test]
    fn test_validate_key_format_consecutive_underscores() {
        let result = validate_key_format("meta__key");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("consecutive underscores"));
    }

    #[test]
    fn test_validate_key_format_empty() {
        let result = validate_key_format("");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty"));
    }
}
