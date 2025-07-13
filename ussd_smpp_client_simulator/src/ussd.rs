use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use log::{debug, warn};
use crate::config::{ClientConfig, MenuOption};

#[derive(Debug, Clone)]
pub struct UssdSession {
    pub msisdn: String,
    pub session_id: String,
    pub current_menu: String,
    pub menu_history: Vec<String>,
    pub last_activity: SystemTime,
    pub menu_depth: u32,
    pub data: HashMap<String, String>, // For storing user inputs
}

impl UssdSession {
    pub fn new(msisdn: String) -> Self {
        UssdSession {
            msisdn,
            session_id: generate_session_id(),
            current_menu: "main".to_string(),
            menu_history: Vec::new(),
            last_activity: SystemTime::now(),
            menu_depth: 0,
            data: HashMap::new(),
        }
    }

    pub fn update_last_activity(&mut self) {
        self.last_activity = SystemTime::now();
    }

    pub fn is_expired(&self, timeout_seconds: u64) -> bool {
        if let Ok(duration) = self.last_activity.elapsed() {
            duration.as_secs() > timeout_seconds
        } else {
            false
        }
    }

    pub fn navigate_to_menu(&mut self, menu_name: &str) {
        if menu_name != self.current_menu {
            self.menu_history.push(self.current_menu.clone());
            self.current_menu = menu_name.to_string();
            self.menu_depth += 1;
        }
    }

    pub fn go_back(&mut self) -> bool {
        if let Some(previous_menu) = self.menu_history.pop() {
            self.current_menu = previous_menu;
            self.menu_depth = self.menu_depth.saturating_sub(1);
            true
        } else {
            false
        }
    }

    pub fn reset_to_main(&mut self, default_menu: &str) {
        self.current_menu = default_menu.to_string();
        self.menu_history.clear();
        self.menu_depth = 0;
        self.data.clear();
    }
}

#[derive(Debug)]
pub struct UssdMenuManager {
    config: ClientConfig,
}

impl UssdMenuManager {
    pub fn new(config: ClientConfig) -> Self {
        UssdMenuManager { config }
    }

    pub fn process_input(&self, session: &mut UssdSession, input: &str) -> String {
        let input = input.trim();
        
        debug!("🔍 UssdMenuManager::process_input called with input: '{}'", input);
        
        // Check for session timeout
        if session.is_expired(self.config.session.timeout_seconds) {
            debug!("⏰ Session expired, resetting to main menu");
            session.reset_to_main(&self.config.menus.default_menu);
            return self.config.responses.defaults.session_timeout.clone();
        }

        debug!("🔍 Processing input '{}' for session {} in menu '{}'", 
            input, session.session_id, session.current_menu);

        // Handle new USSD codes (starts with * and ends with #)
        if input.starts_with('*') && input.ends_with('#') {
            debug!("🔍 Input is a USSD code, handling...");
            return self.handle_ussd_code(session, input);
        }

        // Handle back navigation
        if input == "00" && self.config.session.enable_back_navigation {
            debug!("🔍 Back navigation requested");
            if session.go_back() {
                return self.show_menu(session, &session.current_menu.clone());
            } else {
                return self.config.responses.defaults.exit_message.clone();
            }
        }

        debug!("🔍 Looking for menu option in current menu: {}", session.current_menu);

        // Get current menu
        let current_menu_name = session.current_menu.clone();
        if let Some(menu) = self.config.menus.menus.get(&current_menu_name) {
            debug!("✅ Found menu: {}", current_menu_name);
            // Find matching option
            if let Some(option) = menu.options.iter().find(|opt| opt.key == input) {
                debug!("✅ Found matching option: {} -> {}", option.key, option.action);
                return self.handle_menu_option(session, option);
            } else {
                debug!("❌ No matching option found for input: {}", input);
                // Invalid option
                let mut response = self.config.responses.defaults.invalid_option.clone();
                response.push_str("\n\n");
                response.push_str(&self.show_menu(session, &current_menu_name));
                return response;
            }
        }

        // Menu not found
        warn!("❌ Menu '{}' not found", current_menu_name);
        session.reset_to_main(&self.config.menus.default_menu);
        self.config.responses.defaults.system_error.clone()
    }

    fn handle_menu_option(&self, session: &mut UssdSession, option: &MenuOption) -> String {
        debug!("🎯 Handling option: {} -> {}", option.key, option.action);

        match option.action.as_str() {
            "submenu" => {
                // Navigate to submenu
                if option.target.is_empty() {
                    return self.config.responses.defaults.system_error.clone();
                }

                // Check max depth
                if session.menu_depth >= self.config.session.max_menu_depth {
                    return format!("❌ Maximum menu depth reached.\n\n{}", 
                        self.config.responses.defaults.invalid_option);
                }

                session.navigate_to_menu(&option.target);
                self.show_menu(session, &option.target)
            }
            "response" => {
                // Show response
                if let Some(response) = self.config.responses.responses.get(&option.target) {
                    response.clone()
                } else {
                    warn!("❌ Response '{}' not found", option.target);
                    self.config.responses.defaults.system_error.clone()
                }
            }
            "exit" => {
                // Exit session
                session.reset_to_main(&self.config.menus.default_menu);
                self.config.responses.defaults.exit_message.clone()
            }
            _ => {
                warn!("❌ Unknown action: {}", option.action);
                self.config.responses.defaults.system_error.clone()
            }
        }
    }

    fn show_menu(&self, session: &UssdSession, menu_name: &str) -> String {
        if let Some(menu) = self.config.menus.menus.get(menu_name) {
            let mut response = format!("{}\n\n", menu.title);
            
            for option in &menu.options {
                response.push_str(&format!("{}. {}\n", option.key, option.text));
            }

            // Add navigation help
            if self.config.session.enable_back_navigation && session.menu_depth > 0 {
                response.push_str("\n00. 🔙 Back");
            }

            response
        } else {
            warn!("❌ Menu '{}' not found", menu_name);
            self.config.responses.defaults.system_error.clone()
        }
    }

    pub fn get_welcome_message(&self) -> String {
        self.show_menu(&UssdSession::new("temp".to_string()), &self.config.menus.default_menu)
    }

    pub fn cleanup_expired_sessions(&self, sessions: &mut HashMap<String, UssdSession>) {
        let timeout = self.config.session.timeout_seconds;
        let expired_keys: Vec<String> = sessions
            .iter()
            .filter(|(_, session)| session.is_expired(timeout))
            .map(|(key, _)| key.clone())
            .collect();

        for key in expired_keys {
            sessions.remove(&key);
            debug!("🗑️ Removed expired session: {}", key);
        }
    }

    fn handle_ussd_code(&self, session: &mut UssdSession, ussd_code: &str) -> String {
        debug!("🔍 Handling USSD code: {}", ussd_code);
        
        // Check if this client should handle this USSD code
        if !self.config.ussd_codes.handle_codes.is_empty() {
            if !self.config.ussd_codes.handle_codes.contains(&ussd_code.to_string()) {
                debug!("🚫 USSD code {} not in handle_codes list", ussd_code);
                return self.handle_unrecognized_code(ussd_code);
            }
        }

        // Look for specific mapping for this USSD code
        for mapping in &self.config.ussd_codes.codes {
            if mapping.code == ussd_code {
                debug!("✅ Found mapping for USSD code {} -> menu {}", ussd_code, mapping.menu);
                session.reset_to_main(&mapping.menu);
                return self.show_menu(session, &mapping.menu);
            }
        }

        // No specific mapping found, use default menu
        debug!("📝 No specific mapping for USSD code {}, using default menu", ussd_code);
        let default_menu = &self.config.ussd_codes.default_menu;
        session.reset_to_main(default_menu);
        self.show_menu(session, default_menu)
    }

    fn handle_unrecognized_code(&self, ussd_code: &str) -> String {
        debug!("🚫 Handling unrecognized USSD code: {}", ussd_code);
        
        match self.config.ussd_codes.unrecognized_action.as_str() {
            "reject" => {
                format!("🚫 USSD code {} is not supported by this service.\n\n{}", 
                    ussd_code, self.config.ussd_codes.unrecognized_message)
            }
            "default_menu" => {
                format!("⚠️ USSD code {} redirected to main menu.\n\n{}", 
                    ussd_code, self.config.ussd_codes.unrecognized_message)
            }
            "forward" | _ => {
                // In a real implementation, this would forward to the actual USSD gateway
                // For now, we'll show a message
                format!("🔄 USSD code {} forwarded to network.\n\n{}", 
                    ussd_code, self.config.ussd_codes.unrecognized_message)
            }
        }
    }

    pub fn get_supported_ussd_codes(&self) -> Vec<String> {
        if self.config.ussd_codes.handle_codes.is_empty() {
            self.config.ussd_codes.codes.iter().map(|c| c.code.clone()).collect()
        } else {
            self.config.ussd_codes.handle_codes.clone()
        }
    }

    pub fn get_ussd_code_description(&self, code: &str) -> Option<String> {
        self.config.ussd_codes.codes.iter()
            .find(|mapping| mapping.code == code)
            .map(|mapping| mapping.description.clone())
    }
}

fn generate_session_id() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    format!("USSD{}", timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ClientConfig;

    #[test]
    fn test_session_creation() {
        let session = UssdSession::new("1234567890".to_string());
        assert_eq!(session.msisdn, "1234567890");
        assert_eq!(session.current_menu, "main");
        assert_eq!(session.menu_depth, 0);
    }

    #[test]
    fn test_menu_navigation() {
        let mut session = UssdSession::new("1234567890".to_string());
        
        session.navigate_to_menu("banking");
        assert_eq!(session.current_menu, "banking");
        assert_eq!(session.menu_depth, 1);
        assert_eq!(session.menu_history.len(), 1);

        let went_back = session.go_back();
        assert!(went_back);
        assert_eq!(session.current_menu, "main");
        assert_eq!(session.menu_depth, 0);
    }

    #[test]
    fn test_session_timeout() {
        let mut session = UssdSession::new("1234567890".to_string());
        
        // Session should not be expired immediately
        assert!(!session.is_expired(300));
        
        // Simulate old timestamp
        session.last_activity = SystemTime::now() - std::time::Duration::from_secs(400);
        assert!(session.is_expired(300));
    }
}
