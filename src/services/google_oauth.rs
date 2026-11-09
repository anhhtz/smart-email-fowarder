use crate::models::GoogleOAuthConfig;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{error, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleUserInfo {
    pub id: String,
    pub email: String,
    pub verified_email: Option<bool>,
    pub name: Option<String>,
    pub picture: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    token_type: Option<String>,
    #[serde(default)]
    expires_in: Option<u64>,
    #[serde(default)]
    id_token: Option<String>,
    #[serde(default)]
    refresh_token: Option<String>,
}

pub struct GoogleOAuthService;

impl GoogleOAuthService {
    /// Read a key from .env file directly (so changes on disk apply immediately without restarting server),
    /// with fallback to process environment variables.
    pub fn get_env_var(key: &str) -> Option<String> {
        // In test mode, do not read from workspace .env file to ensure test isolation
        if std::env::var("SMART_EMAIL_TEST").is_err() {
            if let Ok(content) = std::fs::read_to_string(".env") {
                for line in content.lines() {
                    let line = line.trim();
                    if line.starts_with('#') || line.is_empty() {
                        continue;
                    }
                    if let Some((k, v)) = line.split_once('=') {
                        if k.trim() == key {
                            let val = v.trim().trim_matches('"').trim_matches('\'').trim().to_string();
                            if !val.is_empty() {
                                return Some(val);
                            }
                        }
                    }
                }
            }
        }
        std::env::var(key).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
    }

    /// Load effective configuration, prioritizing environment variables (.env)
    pub fn get_config(db_cfg: Option<&GoogleOAuthConfig>) -> GoogleOAuthConfig {
        // Read from environment / .env file directly
        let env_client_id = Self::get_env_var("GOOGLE_CLIENT_ID")
            .filter(|s| s != "your_google_client_id_here");

        let env_client_secret = Self::get_env_var("GOOGLE_CLIENT_SECRET")
            .filter(|s| s != "your_google_client_secret_here");

        let env_redirect_uri = Self::get_env_var("GOOGLE_REDIRECT_URI");

        let client_id = env_client_id.or_else(|| db_cfg.and_then(|c| c.client_id.clone()));
        let client_secret = env_client_secret.or_else(|| db_cfg.and_then(|c| c.client_secret.clone()));
        let mut redirect_uri = env_redirect_uri
            .or_else(|| db_cfg.map(|c| c.redirect_uri.clone()))
            .unwrap_or_else(|| "http://localhost:8080/api/auth/oauth/google/callback".to_string());
        if redirect_uri.starts_with('/') {
            redirect_uri = format!("http://localhost:8080{}", redirect_uri);
        }

        let env_scopes = Self::get_env_var("GOOGLE_SCOPES")
            .map(|s| {
                s.split(|c: char| c == ',' || c == ' ')
                    .map(|part| part.trim().to_string())
                    .filter(|part| !part.is_empty())
                    .collect::<Vec<String>>()
            })
            .filter(|v| !v.is_empty());

        let scopes = env_scopes
            .or_else(|| db_cfg.map(|c| {
                // Filter out https://mail.google.com/ from legacy DB configs if present
                c.scopes.iter()
                    .filter(|s| *s != "https://mail.google.com/")
                    .cloned()
                    .collect()
            }))
            .filter(|v: &Vec<String>| !v.is_empty())
            .unwrap_or_else(|| vec![
                "openid".to_string(),
                "email".to_string(),
                "profile".to_string(),
            ]);

        let is_configured = client_id.is_some() && client_secret.is_some();

        GoogleOAuthConfig {
            enabled: is_configured,
            client_id,
            client_secret,
            redirect_uri,
            scopes,
            auth_url_preview: None,
        }
    }

    pub fn is_configured(cfg: &GoogleOAuthConfig) -> bool {
        match (&cfg.client_id, &cfg.client_secret) {
            (Some(cid), Some(sec)) => {
                let cid = cid.trim();
                let sec = sec.trim();
                !cid.is_empty()
                    && !sec.is_empty()
                    && cid != "your_google_client_id_here"
                    && sec != "your_google_client_secret_here"
            }
            _ => false,
        }
    }

    /// Normalize email for consistent comparison:
    /// - Lowercase all characters and trim whitespace
    /// - For Gmail/Googlemail: ignore dots in local part and ignore +tags
    pub fn normalize_email(email: &str) -> String {
        let email = email.trim().to_lowercase();
        if let Some((user, domain)) = email.split_once('@') {
            let domain = domain.trim();
            let user = user.trim();
            if domain == "gmail.com" || domain == "googlemail.com" {
                let user_base = user.split('+').next().unwrap_or(user).replace('.', "");
                format!("{}@gmail.com", user_base)
            } else {
                let user_base = user.split('+').next().unwrap_or(user);
                format!("{}@{}", user_base, domain)
            }
        } else {
            email
        }
    }

    /// Check if an email is permitted by the ALLOWED_EMAILS environment variable whitelist.
    /// If ALLOWED_EMAILS is unset or empty, all Google authenticated emails are permitted.
    pub fn is_email_allowed(email: &str) -> bool {
        let allowed_raw = match Self::get_env_var("ALLOWED_EMAILS") {
            Some(val) => val,
            None => return true,
        };

        let trimmed = allowed_raw.trim();
        if trimmed.is_empty() {
            return true;
        }

        let normalized_target = Self::normalize_email(email);
        trimmed
            .split(|c: char| c == ',' || c == ';' || c.is_whitespace())
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| Self::normalize_email(s))
            .any(|allowed| allowed == normalized_target)
    }

    pub fn get_allowed_emails() -> Option<Vec<String>> {
        Self::get_env_var("ALLOWED_EMAILS")
            .map(|val| {
                val.split(|c: char| c == ',' || c == ';' || c.is_whitespace())
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .collect()
            })
            .filter(|v: &Vec<String>| !v.is_empty())
    }

    pub fn build_authorization_url(cfg: &GoogleOAuthConfig, state: &str) -> Result<String, String> {
        let client_id = cfg.client_id.as_deref().ok_or_else(|| {
            "GOOGLE_CLIENT_ID is not configured. Please set GOOGLE_CLIENT_ID in your .env file."
                .to_string()
        })?;

        let redirect_uri = &cfg.redirect_uri;
        let scopes = cfg.scopes.join(" ");

        let encoded_redirect = percent_encode(redirect_uri);
        let encoded_scopes = percent_encode(&scopes);
        let encoded_state = percent_encode(state);

        Ok(format!(
            "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}&access_type=offline&prompt=select_account",
            client_id, encoded_redirect, encoded_scopes, encoded_state
        ))
    }

    pub async fn exchange_code_for_user_info(
        cfg: &GoogleOAuthConfig,
        code: &str,
    ) -> Result<GoogleUserInfo, String> {
        let client_id = cfg.client_id.as_deref().ok_or_else(|| {
            "GOOGLE_CLIENT_ID is not configured in .env".to_string()
        })?;
        let client_secret = cfg.client_secret.as_deref().ok_or_else(|| {
            "GOOGLE_CLIENT_SECRET is not configured in .env".to_string()
        })?;

        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        // 1. Exchange code for tokens
        let token_params = [
            ("code", code),
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("redirect_uri", &cfg.redirect_uri),
            ("grant_type", "authorization_code"),
        ];

        let token_resp = http_client
            .post("https://oauth2.googleapis.com/token")
            .form(&token_params)
            .send()
            .await
            .map_err(|e| format!("Network request to Google token endpoint failed: {}", e))?;

        if !token_resp.status().is_success() {
            let status = token_resp.status();
            let err_body = token_resp
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            error!("Google token exchange failed ({status}): {err_body}");
            return Err(format!(
                "Google OAuth token exchange failed (HTTP {}): {}",
                status, err_body
            ));
        }

        let token_data: TokenResponse = token_resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse Google token response: {}", e))?;

        // 2. Fetch user information using access token
        let userinfo_resp = http_client
            .get("https://www.googleapis.com/oauth2/v2/userinfo")
            .bearer_auth(&token_data.access_token)
            .send()
            .await
            .map_err(|e| format!("Network request to Google UserInfo endpoint failed: {}", e))?;

        if !userinfo_resp.status().is_success() {
            let status = userinfo_resp.status();
            let err_body = userinfo_resp
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!(
                "Google UserInfo request failed (HTTP {}): {}",
                status, err_body
            ));
        }

        let user_info: GoogleUserInfo = userinfo_resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse Google userinfo response: {}", e))?;

        info!("Google OAuth2 authentication verified for: {}", user_info.email);
        Ok(user_info)
    }
}

pub fn percent_encode(s: &str) -> String {
    let mut encoded = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(b as char);
            }
            _ => {
                encoded.push_str(&format!("%{:02X}", b));
            }
        }
    }
    encoded
}
