//! JWT configuration loaded from the environment.

use crate::{env_int, env_or};

pub const DEFAULT_JWT_SECRET: &str = "change-me-in-production-please-32bytes-min";

#[derive(Clone)]
pub struct JwtConfig {
    pub secret: String,
    pub issuer: String,
    pub access_ttl_secs: i64,
    pub refresh_ttl_secs: i64,
}

impl JwtConfig {
    pub fn from_env() -> Self {
        Self {
            secret: env_or("JWT_SECRET", DEFAULT_JWT_SECRET),
            issuer: env_or("JWT_ISSUER", "iam-auth"),
            access_ttl_secs: env_int("ACCESS_TOKEN_TTL", 900),
            refresh_ttl_secs: env_int("REFRESH_TOKEN_TTL", 604800),
        }
    }
}

/// Whether APP_ENV indicates production.
pub fn is_production() -> bool {
    matches!(env_or("APP_ENV", "development").as_str(), "production" | "prod")
}

/// Shared gateway→service token (empty disables enforcement; local dev).
pub fn internal_token() -> String {
    env_or("INTERNAL_SERVICE_TOKEN", "")
}

/// Fail fast on insecure configuration in production.
pub fn validate_security() -> Result<(), String> {
    if !is_production() {
        return Ok(());
    }
    let secret = env_or("JWT_SECRET", DEFAULT_JWT_SECRET);
    if secret == DEFAULT_JWT_SECRET || secret.len() < 32 {
        return Err("JWT_SECRET must be a strong value (>=32 bytes) in production".into());
    }
    if env_or("BOOTSTRAP_ADMIN_PASSWORD", "") == "admin12345" {
        return Err("BOOTSTRAP_ADMIN_PASSWORD must not be the default in production".into());
    }
    if internal_token().is_empty() {
        return Err("INTERNAL_SERVICE_TOKEN must be set in production".into());
    }
    Ok(())
}
