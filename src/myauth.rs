use dotenvy::dotenv;
use serde::Deserialize;
use serenity::prelude::*;
use std::env;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct AuthData {
    #[serde(default)]
    pub giphy: String,
    #[serde(default)]
    pub discord: String,
}

impl TypeMapKey for AuthData {
    type Value = AuthData;
}

impl AuthData {
    pub fn load() -> Result<Self, String> {
        dotenv().ok();

        let mut discord = env::var("DISCORD_TOKEN").ok();
        let mut giphy = env::var("GIPHY_TOKEN").ok().unwrap_or_default();

        // If either discord or giphy token is missing from env, check auth.json or configure.json
        if discord.is_none() || giphy.is_empty() {
            for filename in &["auth.json", "configure.json"] {
                if Path::new(filename).exists() {
                    if let Ok(content) = fs::read_to_string(filename) {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                            if discord.is_none() {
                                if let Some(t) = json
                                    .get("token")
                                    .or_else(|| json.get("discord"))
                                    .or_else(|| json.get("discord_token"))
                                    .or_else(|| json.get("discordToken"))
                                    .and_then(|v| v.as_str())
                                {
                                    discord = Some(t.to_string());
                                }
                            }
                            if giphy.is_empty() {
                                if let Some(g) = json
                                    .get("giphy")
                                    .or_else(|| json.get("giphy_token"))
                                    .or_else(|| json.get("giphyToken"))
                                    .and_then(|v| v.as_str())
                                {
                                    giphy = g.to_string();
                                }
                            }
                        }
                    }
                }
            }
        }

        match discord {
            Some(d) if !d.trim().is_empty() => Ok(AuthData {
                discord: d.trim().to_string(),
                giphy: giphy.trim().to_string(),
            }),
            _ => Err(
                "Discord token missing! Please specify DISCORD_TOKEN in environment variables, .env file, or provide auth.json with a 'token' property."
                    .to_string(),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_deserialize_auth_json_legacy_format() {
        let legacy = r#"{"token": "test_token_123", "giphy": "giphy_key_abc"}"#;
        let json = serde_json::from_str::<serde_json::Value>(legacy).unwrap();
        let token = json.get("token").and_then(|v| v.as_str()).unwrap();
        let giphy = json.get("giphy").and_then(|v| v.as_str()).unwrap();
        assert_eq!(token, "test_token_123");
        assert_eq!(giphy, "giphy_key_abc");
    }

    #[test]
    fn test_deserialize_auth_json_alt_keys() {
        let alt = r#"{"discordToken": "tok_xyz", "giphyToken": "gip_xyz"}"#;
        let json = serde_json::from_str::<serde_json::Value>(alt).unwrap();
        let token = json
            .get("discordToken")
            .and_then(|v| v.as_str())
            .unwrap();
        let giphy = json
            .get("giphyToken")
            .and_then(|v| v.as_str())
            .unwrap();
        assert_eq!(token, "tok_xyz");
        assert_eq!(giphy, "gip_xyz");
    }
}
