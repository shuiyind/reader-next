use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub server_host: String,
    pub server_port: u16,
    pub database_url: String,
    pub storage_dir: String,
    pub web_root: String,
    pub assets_dir: String,
    pub log_level: String,
    pub request_timeout_secs: u64,
    pub cache_max_bytes: u64,
    pub cache_max_age_days: u64,
    pub secure: bool,
    pub secure_key: String,
    pub invite_code: String,
    pub user_limit: u32,
    pub user_book_limit: u32,
}

impl AppConfig {
    pub fn validate(&self) -> anyhow::Result<()> {
        if !self.secure_key.is_empty() && self.secure_key.len() < 32 {
            anyhow::bail!("SECURE_KEY 至少需要32个字符");
        }
        if !self.invite_code.is_empty() && self.invite_code.len() < 16 {
            anyhow::bail!("INVITE_CODE 至少需要16个字符");
        }
        Ok(())
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server_host: "127.0.0.1".to_string(),
            server_port: 18080,
            database_url: "sqlite:storage/reader.db?mode=rwc".to_string(),
            storage_dir: "storage".to_string(),
            web_root: "frontend/dist".to_string(),
            assets_dir: "storage/assets".to_string(),
            log_level: "info".to_string(),
            request_timeout_secs: 15,
            cache_max_bytes: crate::storage::cache::file_cache::DEFAULT_CACHE_MAX_BYTES,
            cache_max_age_days: 30,
            secure: true,
            secure_key: "".to_string(),
            invite_code: "".to_string(),
            user_limit: 50,
            user_book_limit: 2000,
        }
    }
}

pub fn load() -> anyhow::Result<AppConfig> {
    dotenvy::dotenv().ok();
    let defaults = AppConfig::default();
    let cfg = config::Config::builder()
        .set_default("server_host", defaults.server_host)?
        .set_default("server_port", defaults.server_port as i64)?
        .set_default("database_url", defaults.database_url)?
        .set_default("storage_dir", defaults.storage_dir)?
        .set_default("web_root", defaults.web_root)?
        .set_default("assets_dir", defaults.assets_dir)?
        .set_default("log_level", defaults.log_level)?
        .set_default("request_timeout_secs", defaults.request_timeout_secs as i64)?
        .set_default("cache_max_bytes", defaults.cache_max_bytes as i64)?
        .set_default("cache_max_age_days", defaults.cache_max_age_days as i64)?
        .set_default("secure", defaults.secure)?
        .set_default("secure_key", defaults.secure_key)?
        .set_default("invite_code", defaults.invite_code)?
        .set_default("user_limit", defaults.user_limit as i64)?
        .set_default("user_book_limit", defaults.user_book_limit as i64)?
        .add_source(config::Environment::default().try_parsing(true))
        .build()?;
    let config: AppConfig = cfg.try_deserialize()?;
    config.validate()?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_short_production_secrets() {
        let mut config = AppConfig::default();
        config.secure_key = "short".to_string();
        assert!(config.validate().is_err());

        let mut config = AppConfig::default();
        config.invite_code = "short".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn accepts_empty_or_strong_production_secrets() {
        let mut config = AppConfig::default();
        config.secure_key = "a".repeat(32);
        config.invite_code = "b".repeat(16);
        assert!(config.validate().is_ok());
    }
}
