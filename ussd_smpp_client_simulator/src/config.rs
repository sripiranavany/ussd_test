use std::collections::HashMap;
use std::fs;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ClientConfig {
    pub client: ClientSettings,
    pub logging: LoggingConfig,
    pub ussd_codes: UssdCodeConfig,
    pub menus: MenuConfigs,
    pub responses: ResponseConfigs,
    pub session: SessionConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ClientSettings {
    pub host: String,
    pub port: u16,
    pub system_id: String,
    pub password: String,
    pub bind_type: String,
    pub auto_reconnect: bool,
    pub heartbeat_interval: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoggingConfig {
    pub level: String,
    pub debug: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UssdCodeConfig {
    pub default_menu: String,
    pub codes: Vec<UssdCodeMapping>,
    pub handle_codes: Vec<String>,
    pub unrecognized_action: String,
    pub unrecognized_message: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UssdCodeMapping {
    pub code: String,
    pub menu: String,
    pub description: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MenuConfigs {
    pub default_menu: String,
    #[serde(flatten)]
    pub menus: HashMap<String, MenuConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MenuConfig {
    pub title: String,
    pub options: Vec<MenuOption>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MenuOption {
    pub key: String,
    pub text: String,
    pub action: String, // "submenu", "response", "exit"
    pub target: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ResponseConfigs {
    #[serde(flatten)]
    pub responses: HashMap<String, String>,
    pub defaults: DefaultResponses,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DefaultResponses {
    pub invalid_option: String,
    pub session_timeout: String,
    pub system_error: String,
    pub exit_message: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SessionConfig {
    pub timeout_seconds: u64,
    pub max_menu_depth: u32,
    pub enable_back_navigation: bool,
    pub remember_last_menu: bool,
}

impl ClientConfig {
    pub fn load(path: &str) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: ClientConfig = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn save(&self, path: &str) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }
}

impl Default for ClientConfig {
    fn default() -> Self {
        let mut menus = HashMap::new();
        
        // Default main menu
        menus.insert("main".to_string(), MenuConfig {
            title: "🏠 Main Menu".to_string(),
            options: vec![
                MenuOption {
                    key: "1".to_string(),
                    text: "💰 Services".to_string(),
                    action: "response".to_string(),
                    target: "services".to_string(),
                },
                MenuOption {
                    key: "0".to_string(),
                    text: "❌ Exit".to_string(),
                    action: "exit".to_string(),
                    target: "".to_string(),
                },
            ],
        });

        let mut responses = HashMap::new();
        responses.insert("services".to_string(), "💰 Services available:\n\n1. Account Info\n2. Transactions\n3. Support\n\nReply with your choice.".to_string());

        ClientConfig {
            client: ClientSettings {
                host: "127.0.0.1".to_string(),
                port: 2775,
                system_id: "ForwardingClient".to_string(),
                password: "forward123".to_string(),
                bind_type: "transceiver".to_string(),
                auto_reconnect: true,
                heartbeat_interval: 30,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                debug: false,
            },
            ussd_codes: UssdCodeConfig {
                default_menu: "main".to_string(),
                codes: vec![],
                handle_codes: vec![],
                unrecognized_action: "response".to_string(),
                unrecognized_message: "🔍 Unrecognized USSD code. Please try again.".to_string(),
            },
            menus: MenuConfigs {
                default_menu: "main".to_string(),
                menus,
            },
            responses: ResponseConfigs {
                responses,
                defaults: DefaultResponses {
                    invalid_option: "❌ Invalid option. Please try again.".to_string(),
                    session_timeout: "⏰ Session timeout. Please try again.".to_string(),
                    system_error: "🔧 System error. Please try again later.".to_string(),
                    exit_message: "👋 Goodbye!".to_string(),
                },
            },
            session: SessionConfig {
                timeout_seconds: 300,
                max_menu_depth: 10,
                enable_back_navigation: true,
                remember_last_menu: false,
            },
        }
    }
}
