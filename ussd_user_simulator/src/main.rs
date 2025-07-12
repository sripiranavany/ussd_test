use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::thread;
use std::time::{Duration, Instant};
use std::env;
use std::fs;
use std::path::Path;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use log::{info, warn, error, debug};

// Enhanced Configuration structures
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UserSimulatorConfig {
    pub server: ServerConfig,
    pub authentication: AuthConfig,
    pub phone: PhoneConfig,
    pub ui: UiConfig,
    pub logging: LoggingConfig,
    pub testing: TestingConfig,
    pub advanced: AdvancedConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub connection_timeout_ms: u64,
    pub reconnect_attempts: u32,
    pub keepalive_interval_ms: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AuthConfig {
    pub system_id: String,
    pub password: String,
    pub system_type: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PhoneConfig {
    pub default_msisdn: String,
    pub operator_name: String,
    pub balance: f64,
    pub data_balance: f64,
    pub country_code: String,
    pub network_code: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UiConfig {
    pub animation_delay_ms: u64,
    pub auto_clear_screen: bool,
    pub show_debug_info: bool,
    pub show_performance_stats: bool,
    pub session_timeout_ms: u64,
    pub max_input_length: usize,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LoggingConfig {
    pub debug: bool,
    pub log_file: String,
    pub log_level: String,
    pub enable_file_logging: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TestingConfig {
    pub auto_test_on_startup: bool,
    pub test_scenarios_file: String,
    pub performance_test_enabled: bool,
    pub concurrent_sessions: u32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AdvancedConfig {
    pub smpp_version: String,
    pub enquire_link_interval_ms: u64,
    pub pdu_timeout_ms: u64,
    pub max_concurrent_requests: u32,
}

impl Default for UserSimulatorConfig {
    fn default() -> Self {
        UserSimulatorConfig {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 9090,
                connection_timeout_ms: 5000,
                reconnect_attempts: 3,
                keepalive_interval_ms: 30000,
            },
            authentication: AuthConfig {
                system_id: "USSDMobileUser".to_string(),
                password: "mobile123".to_string(),
                system_type: "USSD".to_string(),
            },
            phone: PhoneConfig {
                default_msisdn: "1234567890".to_string(),
                operator_name: "MyTelecom".to_string(),
                balance: 25.50,
                data_balance: 2.5,
                country_code: "1".to_string(),
                network_code: "001".to_string(),
            },
            ui: UiConfig {
                animation_delay_ms: 800,
                auto_clear_screen: true,
                show_debug_info: false,
                show_performance_stats: true,
                session_timeout_ms: 30000,
                max_input_length: 160,
            },
            logging: LoggingConfig {
                debug: false,
                log_file: "ussd_simulator.log".to_string(),
                log_level: "info".to_string(),
                enable_file_logging: true,
            },
            testing: TestingConfig {
                auto_test_on_startup: false,
                test_scenarios_file: "test_scenarios.toml".to_string(),
                performance_test_enabled: false,
                concurrent_sessions: 1,
            },
            advanced: AdvancedConfig {
                smpp_version: "3.4".to_string(),
                enquire_link_interval_ms: 60000,
                pdu_timeout_ms: 10000,
                max_concurrent_requests: 5,
            },
        }
    }
}

// Performance Statistics
#[derive(Debug, Clone)]
pub struct PerformanceStats {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub avg_response_time_ms: f64,
    pub min_response_time_ms: u64,
    pub max_response_time_ms: u64,
    pub start_time: Instant,
    pub last_request_time: Option<Instant>,
    pub response_times: Vec<u64>,
}

impl PerformanceStats {
    pub fn new() -> Self {
        PerformanceStats {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            avg_response_time_ms: 0.0,
            min_response_time_ms: u64::MAX,
            max_response_time_ms: 0,
            start_time: Instant::now(),
            last_request_time: None,
            response_times: Vec::new(),
        }
    }

    pub fn record_request(&mut self, response_time_ms: u64, success: bool) {
        self.total_requests += 1;
        self.last_request_time = Some(Instant::now());
        
        if success {
            self.successful_requests += 1;
        } else {
            self.failed_requests += 1;
        }
        
        self.response_times.push(response_time_ms);
        
        // Keep only last 1000 response times to prevent memory issues
        if self.response_times.len() > 1000 {
            self.response_times.remove(0);
        }
        
        self.min_response_time_ms = self.min_response_time_ms.min(response_time_ms);
        self.max_response_time_ms = self.max_response_time_ms.max(response_time_ms);
        
        // Calculate average
        if !self.response_times.is_empty() {
            self.avg_response_time_ms = self.response_times.iter().sum::<u64>() as f64 / self.response_times.len() as f64;
        }
    }
    
    pub fn get_success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            (self.successful_requests as f64 / self.total_requests as f64) * 100.0
        }
    }
    
    pub fn get_uptime_seconds(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }
}

// SMPP Command IDs
const BIND_TRANSCEIVER: u32 = 0x00000009;
const BIND_TRANSCEIVER_RESP: u32 = 0x80000009;
const SUBMIT_SM: u32 = 0x00000004;
const SUBMIT_SM_RESP: u32 = 0x80000004;
const DELIVER_SM: u32 = 0x00000005;
const DELIVER_SM_RESP: u32 = 0x80000005;
const ENQUIRE_LINK: u32 = 0x00000015;
const ENQUIRE_LINK_RESP: u32 = 0x80000015;
const UNBIND: u32 = 0x00000006;
const UNBIND_RESP: u32 = 0x80000006;

// SMPP Status Codes
const ESME_ROK: u32 = 0x00000000;

#[derive(Debug, Clone)]
pub struct SmppHeader {
    pub command_length: u32,
    pub command_id: u32,
    pub command_status: u32,
    pub sequence_number: u32,
}

#[derive(Debug, Clone)]
pub struct SmppPdu {
    pub header: SmppHeader,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct MobilePhone {
    pub msisdn: String,
    pub operator: String,
    pub balance: f64,
    pub data_balance: f64,
}

impl MobilePhone {
    pub fn new(msisdn: &str, operator: &str, balance: f64, data_balance: f64) -> Self {
        MobilePhone {
            msisdn: msisdn.to_string(),
            operator: operator.to_string(),
            balance,
            data_balance,
        }
    }
}

pub struct UssdSmppClient {
    stream: Option<TcpStream>,
    sequence_counter: u32,
    bound: bool,
    config: UserSimulatorConfig,
    stats: PerformanceStats,
    connection_start_time: Option<Instant>,
    last_activity: Option<Instant>,
}

impl UssdSmppClient {
    pub fn new(config: UserSimulatorConfig) -> Self {
        UssdSmppClient {
            stream: None,
            sequence_counter: 1,
            bound: false,
            config,
            stats: PerformanceStats::new(),
            connection_start_time: None,
            last_activity: None,
        }
    }

    pub fn connect(&mut self) -> std::io::Result<bool> {
        let server_addr = format!("{}:{}", self.config.server.host, self.config.server.port);
        
        if self.config.logging.debug {
            println!("🔗 Connecting to USSD SMPP server at {}", server_addr);
        }
        
        let start_time = Instant::now();
        
        // Try to connect with timeout
        let stream = match TcpStream::connect_timeout(
            &server_addr.parse().map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?,
            Duration::from_millis(self.config.server.connection_timeout_ms)
        ) {
            Ok(stream) => {
                self.connection_start_time = Some(start_time);
                self.last_activity = Some(Instant::now());
                stream
            },
            Err(e) => {
                if self.config.logging.debug {
                    println!("❌ Connection failed: {}", e);
                }
                return Err(e);
            }
        };
        
        // Set socket options for better performance
        if let Err(e) = stream.set_nodelay(true) {
            if self.config.logging.debug {
                println!("⚠️  Warning: Could not set TCP_NODELAY: {}", e);
            }
        }
        
        self.stream = Some(stream);
        
        // Bind to server
        self.bind()
    }

    pub fn reconnect(&mut self) -> std::io::Result<bool> {
        if self.config.logging.debug {
            println!("🔄 Attempting to reconnect...");
        }
        
        self.disconnect();
        
        for attempt in 1..=self.config.server.reconnect_attempts {
            if self.config.logging.debug {
                println!("🔄 Reconnection attempt {}/{}", attempt, self.config.server.reconnect_attempts);
            }
            
            match self.connect() {
                Ok(true) => {
                    if self.config.logging.debug {
                        println!("✅ Reconnected successfully");
                    }
                    return Ok(true);
                },
                Ok(false) => {
                    if self.config.logging.debug {
                        println!("❌ Reconnection failed (bind failed)");
                    }
                },
                Err(e) => {
                    if self.config.logging.debug {
                        println!("❌ Reconnection failed: {}", e);
                    }
                }
            }
            
            if attempt < self.config.server.reconnect_attempts {
                thread::sleep(Duration::from_millis(1000 * attempt as u64));
            }
        }
        
        Ok(false)
    }

    pub fn disconnect(&mut self) {
        if self.bound {
            let _ = self.unbind();
        }
        self.stream = None;
        self.bound = false;
        self.connection_start_time = None;
        self.last_activity = None;
    }

    pub fn is_connected(&self) -> bool {
        self.stream.is_some() && self.bound
    }

    pub fn get_stats(&self) -> &PerformanceStats {
        &self.stats
    }

    pub fn get_connection_uptime_seconds(&self) -> Option<u64> {
        self.connection_start_time.map(|start| start.elapsed().as_secs())
    }

    fn bind(&mut self) -> std::io::Result<bool> {
        if self.config.logging.debug {
            println!("🔐 Binding with system_id: {}", self.config.authentication.system_id);
        }
        
        let mut body = Vec::new();
        body.extend_from_slice(self.config.authentication.system_id.as_bytes());
        body.push(0); // null terminator
        body.extend_from_slice(self.config.authentication.password.as_bytes());
        body.push(0); // null terminator
        body.extend_from_slice(self.config.authentication.system_type.as_bytes());
        body.push(0); // null terminator
        body.push(0x34); // interface_version (3.4)
        body.push(1); // addr_ton
        body.push(1); // addr_npi
        body.extend_from_slice(b"\0"); // address_range

        let bind_pdu = SmppPdu {
            header: SmppHeader {
                command_length: 16 + body.len() as u32,
                command_id: BIND_TRANSCEIVER,
                command_status: ESME_ROK,
                sequence_number: self.get_next_sequence(),
            },
            body,
        };

        let start_time = Instant::now();
        self.send_pdu(bind_pdu)?;
        
        // Wait for bind response with timeout
        let response = self.read_pdu_with_timeout(Duration::from_millis(self.config.advanced.pdu_timeout_ms))?;
        let response_time = start_time.elapsed().as_millis() as u64;
        
        if response.header.command_id == BIND_TRANSCEIVER_RESP && response.header.command_status == ESME_ROK {
            self.bound = true;
            self.last_activity = Some(Instant::now());
            if self.config.logging.debug {
                println!("✅ Bind successful ({}ms)", response_time);
            }
            Ok(true)
        } else {
            if self.config.logging.debug {
                println!("❌ Bind failed. Status: 0x{:08x} ({}ms)", response.header.command_status, response_time);
            }
            Ok(false)
        }
    }

    pub fn send_ussd_request(&mut self, ussd_code: &str) -> std::io::Result<String> {
        if !self.bound {
            return Err(std::io::Error::new(std::io::ErrorKind::NotConnected, "Not bound to server"));
        }

        if self.config.logging.debug {
            println!("📤 Sending USSD request: {}", ussd_code);
        }

        let start_time = Instant::now();
        let mut body = Vec::new();
        body.extend_from_slice(b"USSD\0"); // service_type
        body.push(1); // source_addr_ton (International)
        body.push(1); // source_addr_npi (ISDN)
        body.extend_from_slice(self.config.phone.default_msisdn.as_bytes()); // source_addr
        body.push(0); // null terminator
        body.push(0); // dest_addr_ton
        body.push(0); // dest_addr_npi
        body.extend_from_slice(b"123\0"); // destination_addr (USSD gateway)
        body.push(0x40); // esm_class (USSD indication)
        body.push(0); // protocol_id
        body.push(0); // priority_flag
        body.extend_from_slice(b"\0"); // schedule_delivery_time
        body.extend_from_slice(b"\0"); // validity_period
        body.push(0); // registered_delivery
        body.push(0); // replace_if_present_flag
        body.push(0); // data_coding (GSM 7-bit)
        body.push(0); // sm_default_msg_id
        body.push(ussd_code.len() as u8); // sm_length
        body.extend_from_slice(ussd_code.as_bytes()); // short_message

        let submit_pdu = SmppPdu {
            header: SmppHeader {
                command_length: 16 + body.len() as u32,
                command_id: SUBMIT_SM,
                command_status: ESME_ROK,
                sequence_number: self.get_next_sequence(),
            },
            body,
        };

        self.send_pdu(submit_pdu)?;

        // Wait for submit response
        let submit_resp = self.read_pdu_with_timeout(Duration::from_millis(self.config.advanced.pdu_timeout_ms))?;
        let success = submit_resp.header.command_id == SUBMIT_SM_RESP && submit_resp.header.command_status == ESME_ROK;
        
        if success {
            if self.config.logging.debug {
                println!("✅ SUBMIT_SM_RESP received");
            }
            
            // Wait for DELIVER_SM with USSD response
            let deliver_sm = self.read_pdu_with_timeout(Duration::from_millis(self.config.ui.session_timeout_ms))?;
            if deliver_sm.header.command_id == DELIVER_SM {
                let response_text = self.parse_deliver_sm(&deliver_sm.body);
                
                // Send DELIVER_SM_RESP
                let deliver_resp = SmppPdu {
                    header: SmppHeader {
                        command_length: 16,
                        command_id: DELIVER_SM_RESP,
                        command_status: ESME_ROK,
                        sequence_number: deliver_sm.header.sequence_number,
                    },
                    body: Vec::new(),
                };
                self.send_pdu(deliver_resp)?;
                
                let total_time = start_time.elapsed().as_millis() as u64;
                self.stats.record_request(total_time, true);
                self.last_activity = Some(Instant::now());
                
                if self.config.logging.debug {
                    println!("📥 USSD response received: {} ({}ms)", response_text, total_time);
                }
                
                Ok(response_text)
            } else {
                let total_time = start_time.elapsed().as_millis() as u64;
                self.stats.record_request(total_time, false);
                Err(std::io::Error::new(std::io::ErrorKind::Other, "Expected DELIVER_SM"))
            }
        } else {
            let total_time = start_time.elapsed().as_millis() as u64;
            self.stats.record_request(total_time, false);
            Err(std::io::Error::new(std::io::ErrorKind::Other, "SUBMIT_SM failed"))
        }
    }

    pub fn unbind(&mut self) -> std::io::Result<()> {
        if !self.bound {
            return Ok(());
        }

        let unbind_pdu = SmppPdu {
            header: SmppHeader {
                command_length: 16,
                command_id: UNBIND,
                command_status: ESME_ROK,
                sequence_number: self.get_next_sequence(),
            },
            body: Vec::new(),
        };

        self.send_pdu(unbind_pdu)?;
        
        // Wait for unbind response
        let _response = self.read_pdu()?;
        self.bound = false;
        
        if self.config.logging.debug {
            println!("✅ Unbind successful");
        }
        
        Ok(())
    }

    fn parse_deliver_sm(&self, body: &[u8]) -> String {
        let mut pos = 0;
        
        // Skip service_type
        while pos < body.len() && body[pos] != 0 { pos += 1; }
        pos += 1;
        
        // Skip source_addr_ton, source_addr_npi
        pos += 2;
        
        // Skip source_addr
        while pos < body.len() && body[pos] != 0 { pos += 1; }
        pos += 1;
        
        // Skip dest_addr_ton, dest_addr_npi
        pos += 2;
        
        // Skip destination_addr
        while pos < body.len() && body[pos] != 0 { pos += 1; }
        pos += 1;
        
        // Skip esm_class, protocol_id, priority_flag
        pos += 3;
        
        // Skip schedule_delivery_time
        while pos < body.len() && body[pos] != 0 { pos += 1; }
        pos += 1;
        
        // Skip validity_period
        while pos < body.len() && body[pos] != 0 { pos += 1; }
        pos += 1;
        
        // Skip registered_delivery, replace_if_present_flag, data_coding, sm_default_msg_id
        pos += 4;
        
        // Get sm_length and short_message
        if pos < body.len() {
            let sm_length = body[pos] as usize;
            pos += 1;
            
            if pos + sm_length <= body.len() {
                return String::from_utf8_lossy(&body[pos..pos + sm_length]).to_string();
            }
        }
        
        String::new()
    }

    fn send_pdu(&mut self, pdu: SmppPdu) -> std::io::Result<()> {
        if let Some(ref mut stream) = self.stream {
            let mut buffer = Vec::new();
            
            buffer.extend_from_slice(&pdu.header.command_length.to_be_bytes());
            buffer.extend_from_slice(&pdu.header.command_id.to_be_bytes());
            buffer.extend_from_slice(&pdu.header.command_status.to_be_bytes());
            buffer.extend_from_slice(&pdu.header.sequence_number.to_be_bytes());
            
            buffer.extend_from_slice(&pdu.body);
            
            stream.write_all(&buffer)?;
            stream.flush()?;
        }
        
        Ok(())
    }

    fn read_pdu(&mut self) -> std::io::Result<SmppPdu> {
        if let Some(ref mut stream) = self.stream {
            let mut header_buf = [0u8; 16];
            stream.read_exact(&mut header_buf)?;

            let command_length = u32::from_be_bytes([header_buf[0], header_buf[1], header_buf[2], header_buf[3]]);
            let command_id = u32::from_be_bytes([header_buf[4], header_buf[5], header_buf[6], header_buf[7]]);
            let command_status = u32::from_be_bytes([header_buf[8], header_buf[9], header_buf[10], header_buf[11]]);
            let sequence_number = u32::from_be_bytes([header_buf[12], header_buf[13], header_buf[14], header_buf[15]]);

            let header = SmppHeader {
                command_length,
                command_id,
                command_status,
                sequence_number,
            };

            let body_length = command_length.saturating_sub(16) as usize;
            let mut body = vec![0u8; body_length];
            if body_length > 0 {
                stream.read_exact(&mut body)?;
            }

            self.last_activity = Some(Instant::now());
            Ok(SmppPdu { header, body })
        } else {
            Err(std::io::Error::new(std::io::ErrorKind::NotConnected, "Not connected"))
        }
    }

    fn read_pdu_with_timeout(&mut self, timeout: Duration) -> std::io::Result<SmppPdu> {
        if let Some(ref mut stream) = self.stream {
            // Set read timeout
            stream.set_read_timeout(Some(timeout))?;
            
            let mut header_buf = [0u8; 16];
            let result = stream.read_exact(&mut header_buf);
            
            // Reset timeout to None (blocking)
            stream.set_read_timeout(None)?;
            
            match result {
                Ok(()) => {
                    let command_length = u32::from_be_bytes([header_buf[0], header_buf[1], header_buf[2], header_buf[3]]);
                    let command_id = u32::from_be_bytes([header_buf[4], header_buf[5], header_buf[6], header_buf[7]]);
                    let command_status = u32::from_be_bytes([header_buf[8], header_buf[9], header_buf[10], header_buf[11]]);
                    let sequence_number = u32::from_be_bytes([header_buf[12], header_buf[13], header_buf[14], header_buf[15]]);

                    let header = SmppHeader {
                        command_length,
                        command_id,
                        command_status,
                        sequence_number,
                    };

                    let body_length = command_length.saturating_sub(16) as usize;
                    let mut body = vec![0u8; body_length];
                    if body_length > 0 {
                        stream.read_exact(&mut body)?;
                    }

                    self.last_activity = Some(Instant::now());
                    Ok(SmppPdu { header, body })
                }
                Err(e) => Err(e)
            }
        } else {
            Err(std::io::Error::new(std::io::ErrorKind::NotConnected, "Not connected"))
        }
    }

    fn get_next_sequence(&mut self) -> u32 {
        self.sequence_counter += 1;
        self.sequence_counter
    }
}

pub struct UssdMobileUI {
    phone: MobilePhone,
    client: UssdSmppClient,
    config: UserSimulatorConfig,
}

impl UssdMobileUI {
    pub fn new(config: UserSimulatorConfig) -> Self {
        let phone = MobilePhone::new(
            &config.phone.default_msisdn,
            &config.phone.operator_name,
            config.phone.balance,
            config.phone.data_balance,
        );
        
        let client = UssdSmppClient::new(config.clone());
        
        UssdMobileUI {
            phone,
            client,
            config,
        }
    }

    pub fn start(&mut self) -> std::io::Result<()> {
        // Connect to SMPP server
        if !self.client.connect()? {
            println!("❌ Failed to connect to USSD server");
            return Err(std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "Connection failed"));
        }

        if self.config.ui.auto_clear_screen {
            self.clear_screen();
        }
        
        self.show_phone_display();
        
        loop {
            self.show_dialer_menu();
            let choice = self.get_user_input()?;
            
            match choice.as_str() {
                "1" => self.dial_ussd("*123#")?,
                "2" => self.dial_ussd("*100#")?,
                "3" => self.dial_ussd("*199#")?,
                "4" => self.custom_ussd()?,
                "5" => self.show_performance_stats()?,
                "6" => self.test_connection()?,
                "7" => self.run_test_scenarios()?,
                "8" => {
                    println!("📱 Goodbye!");
                    break;
                }
                _ => {
                    println!("❌ Invalid choice. Please try again.");
                    thread::sleep(Duration::from_millis(1500));
                }
            }
        }
        
        // Disconnect from server
        self.client.unbind()?;
        
        Ok(())
    }

    fn show_phone_display(&self) {
        println!("╔════════════════════════════════════════╗");
        println!("║              📱 MOBILE PHONE             ║");
        println!("║                                        ║");
        println!("║  📶 {} Signal: ████▓                  ║", self.phone.operator);
        println!("║  📞 {:<30} ║", self.phone.msisdn);
        println!("║  💰 Balance: ${:.2}                    ║", self.phone.balance);
        println!("║  📊 Data: {:.1}GB                        ║", self.phone.data_balance);
        println!("║  🌐 Server: {}:{}                 ║", self.config.server.host, self.config.server.port);
        
        if self.config.ui.show_performance_stats {
            let stats = self.client.get_stats();
            let uptime = self.client.get_connection_uptime_seconds().unwrap_or(0);
            println!("║  📈 Requests: {} (✅{} ❌{})           ║", stats.total_requests, stats.successful_requests, stats.failed_requests);
            println!("║  ⏱️  Avg Response: {:.0}ms             ║", stats.avg_response_time_ms);
            println!("║  🔗 Uptime: {}s                      ║", uptime);
        }
        
        println!("║                                        ║");
        println!("╚════════════════════════════════════════╝");
        println!();
    }

    fn show_dialer_menu(&self) {
        println!("╔════════════════════════════════════════╗");
        println!("║                USSD DIALER             ║");
        println!("║                                        ║");
        println!("║  1. Main Menu (*123#)                  ║");
        println!("║  2. Balance Check (*100#)              ║");
        println!("║  3. Data Balance (*199#)               ║");
        println!("║  4. Custom USSD Code                   ║");
        println!("║  5. Performance Stats                  ║");
        println!("║  6. Connection Test                    ║");
        println!("║  7. Run Test Scenarios                 ║");
        println!("║  8. Exit                               ║");
        println!("║                                        ║");
        println!("╚════════════════════════════════════════╝");
        print!("Enter your choice: ");
        io::stdout().flush().unwrap();
    }

    fn dial_ussd(&mut self, ussd_code: &str) -> std::io::Result<()> {
        if self.config.ui.auto_clear_screen {
            self.clear_screen();
        }
        self.show_phone_display();
        
        println!("╔════════════════════════════════════════╗");
        println!("║                DIALING                 ║");
        println!("║                                        ║");
        println!("║  📞 Dialing: {:<25} ║", ussd_code);
        println!("║                                        ║");
        println!("║  ⏳ Connecting to network...           ║");
        println!("╚════════════════════════════════════════╝");
        
        // Simulate dialing delay
        for _i in 0..3 {
            thread::sleep(Duration::from_millis(self.config.ui.animation_delay_ms));
            print!(".");
            io::stdout().flush().unwrap();
        }
        println!();
        
        // Launch USSD client
        self.launch_ussd_client(ussd_code)?;
        
        // Wait for user to press enter
        println!("\nPress Enter to return to main menu...");
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        if self.config.ui.auto_clear_screen {
            self.clear_screen();
        }
        Ok(())
    }

    fn custom_ussd(&mut self) -> std::io::Result<()> {
        if self.config.ui.auto_clear_screen {
            self.clear_screen();
        }
        self.show_phone_display();
        
        println!("╔════════════════════════════════════════╗");
        println!("║              CUSTOM USSD               ║");
        println!("║                                        ║");
        println!("║  Enter USSD code (e.g., *123#):       ║");
        println!("║                                        ║");
        println!("╚════════════════════════════════════════╝");
        
        let ussd_code = self.get_user_input()?;
        
        if ussd_code.trim().is_empty() {
            println!("❌ No USSD code entered. Returning to menu...");
            thread::sleep(Duration::from_millis(1500));
            return Ok(());
        }
        
        self.dial_ussd(&ussd_code)?;
        Ok(())
    }

    fn launch_ussd_client(&mut self, ussd_code: &str) -> std::io::Result<()> {
        if self.config.ui.auto_clear_screen {
            self.clear_screen();
        }
        self.show_phone_display();
        
        println!("╔════════════════════════════════════════╗");
        println!("║              USSD SESSION              ║");
        println!("║                                        ║");
        println!("║  📞 Code: {:<27} ║", ussd_code);
        println!("║  🔗 Connected to network               ║");
        println!("║                                        ║");
        println!("╚════════════════════════════════════════╝");
        println!();
        
        // Use real USSD interaction
        self.real_ussd_session(ussd_code)?;
        
        Ok(())
    }

    fn real_ussd_session(&mut self, initial_code: &str) -> std::io::Result<()> {
        let mut current_input = initial_code.to_string();
        
        loop {
            println!("┌────────────────────────────────────────┐");
            println!("│              USSD RESPONSE             │");
            println!("└────────────────────────────────────────┘");
            
            // Send real USSD request to server
            match self.client.send_ussd_request(&current_input) {
                Ok(response) => {
                    println!("{}", response);
                    
                    if response.contains("Thank you") || response.contains("Goodbye") || response.contains("Invalid") {
                        println!("\n📱 USSD session ended.");
                        break;
                    }
                    
                    println!("\n┌────────────────────────────────────────┐");
                    println!("│           ENTER YOUR CHOICE            │");
                    println!("└────────────────────────────────────────┘");
                    print!("Your input: ");
                    io::stdout().flush().unwrap();
                    
                    let mut input = String::new();
                    io::stdin().read_line(&mut input)?;
                    current_input = input.trim().to_string();
                    
                    if current_input.is_empty() {
                        println!("📱 USSD session cancelled.");
                        break;
                    }
                    
                    // Show processing animation
                    print!("⏳ Processing");
                    for _i in 0..3 {
                        thread::sleep(Duration::from_millis(500));
                        print!(".");
                        io::stdout().flush().unwrap();
                    }
                    println!();
                }
                Err(e) => {
                    println!("❌ Error: {}", e);
                    println!("📱 USSD session failed.");
                    break;
                }
            }
        }
        
        Ok(())
    }

    fn show_performance_stats(&self) -> std::io::Result<()> {
        if self.config.ui.auto_clear_screen {
            self.clear_screen();
        }
        
        let stats = self.client.get_stats();
        let uptime = self.client.get_connection_uptime_seconds().unwrap_or(0);
        
        println!("╔════════════════════════════════════════╗");
        println!("║           PERFORMANCE STATISTICS       ║");
        println!("║                                        ║");
        println!("║  📊 Total Requests: {:<18} ║", stats.total_requests);
        println!("║  ✅ Successful: {:<22} ║", stats.successful_requests);
        println!("║  ❌ Failed: {:<26} ║", stats.failed_requests);
        println!("║  📈 Success Rate: {:.1}%                ║", stats.get_success_rate());
        println!("║                                        ║");
        println!("║  ⏱️  Average Response: {:.0}ms           ║", stats.avg_response_time_ms);
        println!("║  🚀 Fastest Response: {}ms              ║", if stats.min_response_time_ms == u64::MAX { 0 } else { stats.min_response_time_ms });
        println!("║  🐌 Slowest Response: {}ms              ║", stats.max_response_time_ms);
        println!("║                                        ║");
        println!("║  🔗 Connection Uptime: {}s              ║", uptime);
        println!("║  🌐 Server: {}:{}                 ║", self.config.server.host, self.config.server.port);
        println!("║  📱 MSISDN: {:<25} ║", self.phone.msisdn);
        println!("║                                        ║");
        println!("╚════════════════════════════════════════╝");
        
        println!("\nPress Enter to continue...");
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        if self.config.ui.auto_clear_screen {
            self.clear_screen();
        }
        Ok(())
    }
    
    fn test_connection(&mut self) -> std::io::Result<()> {
        if self.config.ui.auto_clear_screen {
            self.clear_screen();
        }
        
        println!("╔════════════════════════════════════════╗");
        println!("║            CONNECTION TEST             ║");
        println!("║                                        ║");
        println!("║  🔍 Testing SMPP connection...         ║");
        println!("╚════════════════════════════════════════╝");
        
        let start_time = Instant::now();
        
        // Test basic connectivity
        println!("1. Testing TCP connection...");
        if self.client.is_connected() {
            println!("   ✅ Connection active");
        } else {
            println!("   ❌ Connection not active, attempting reconnect...");
            match self.client.reconnect() {
                Ok(true) => println!("   ✅ Reconnection successful"),
                Ok(false) => println!("   ❌ Reconnection failed"),
                Err(e) => println!("   ❌ Reconnection error: {}", e),
            }
        }
        
        // Test USSD request
        println!("2. Testing USSD request...");
        match self.client.send_ussd_request("*000#") {
            Ok(response) => {
                println!("   ✅ USSD test successful");
                println!("   📥 Response: {}", response);
            }
            Err(e) => {
                println!("   ❌ USSD test failed: {}", e);
            }
        }
        
        let total_time = start_time.elapsed();
        println!("\n🎯 Test completed in {:.2}s", total_time.as_secs_f64());
        
        println!("\nPress Enter to continue...");
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        if self.config.ui.auto_clear_screen {
            self.clear_screen();
        }
        Ok(())
    }
    
    fn run_test_scenarios(&mut self) -> std::io::Result<()> {
        if self.config.ui.auto_clear_screen {
            self.clear_screen();
        }
        
        println!("╔════════════════════════════════════════╗");
        println!("║             TEST SCENARIOS             ║");
        println!("║                                        ║");
        println!("║  🧪 Running predefined test scenarios  ║");
        println!("╚════════════════════════════════════════╝");
        
        let scenarios = vec![
            ("*123#", "Main menu test"),
            ("*100#", "Balance check test"),
            ("*199#", "Data balance test"),
            ("*000#", "Network test"),
        ];
        
        let mut passed = 0;
        let mut failed = 0;
        
        for (code, description) in scenarios {
            println!("\n🧪 {}", description);
            print!("   Sending {}... ", code);
            io::stdout().flush().unwrap();
            
            let start_time = Instant::now();
            match self.client.send_ussd_request(code) {
                Ok(response) => {
                    let duration = start_time.elapsed();
                    passed += 1;
                    println!("✅ ({:.0}ms)", duration.as_millis());
                    println!("   📥 {}", response.chars().take(60).collect::<String>());
                    if response.len() > 60 {
                        println!("      [...]");
                    }
                }
                Err(e) => {
                    failed += 1;
                    println!("❌ Failed: {}", e);
                }
            }
            
            thread::sleep(Duration::from_millis(500));
        }
        
        println!("\n╔════════════════════════════════════════╗");
        println!("║              TEST RESULTS              ║");
        println!("║                                        ║");
        println!("║  ✅ Passed: {:<26} ║", passed);
        println!("║  ❌ Failed: {:<26} ║", failed);
        println!("║  📊 Success Rate: {:.1}%                ║", (passed as f64 / (passed + failed) as f64) * 100.0);
        println!("║                                        ║");
        println!("╚════════════════════════════════════════╝");
        
        println!("\nPress Enter to continue...");
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        if self.config.ui.auto_clear_screen {
            self.clear_screen();
        }
        Ok(())
    }
}

fn load_config(config_path: &str) -> Result<UserSimulatorConfig, Box<dyn std::error::Error>> {
    if Path::new(config_path).exists() {
        let config_content = fs::read_to_string(config_path)?;
        let config: UserSimulatorConfig = toml::from_str(&config_content)?;
        Ok(config)
    } else {
        println!("Config file not found at '{}', creating default config...", config_path);
        let default_config = UserSimulatorConfig::default();
        let config_content = toml::to_string_pretty(&default_config)?;
        fs::write(config_path, config_content)?;
        println!("Default config created at '{}'", config_path);
        Ok(default_config)
    }
}

fn print_usage() {
    println!("USSD User Simulator");
    println!("Usage: ussd_user_simulator [OPTIONS]");
    println!();
    println!("Options:");
    println!("  -c, --config <CONFIG>    Path to configuration file (default: user_config.toml)");
    println!("  -m, --msisdn <MSISDN>    Override phone number from config");
    println!("  -h, --host <HOST>        Override server host from config");
    println!("  -p, --port <PORT>        Override server port from config");
    println!("  --create-config          Create a default config file and exit");
    println!("  --debug                  Enable debug mode");
    println!("  --help                   Show this help message");
    println!();
    println!("Examples:");
    println!("  ussd_user_simulator");
    println!("  ussd_user_simulator -c /path/to/config.toml");
    println!("  ussd_user_simulator --msisdn 9876543210 --debug");
    println!("  ussd_user_simulator --host 192.168.1.100");
    println!("  ussd_user_simulator --create-config");
}

fn parse_args() -> Result<(UserSimulatorConfig, Option<String>, Option<String>, Option<u16>), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let mut config_path = "user_config.toml".to_string();
    let mut msisdn_override: Option<String> = None;
    let mut host_override: Option<String> = None;
    let mut port_override: Option<u16> = None;
    let mut debug_override = false;
    
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-c" | "--config" => {
                if i + 1 < args.len() {
                    config_path = args[i + 1].clone();
                    i += 2;
                } else {
                    return Err("--config requires a value".into());
                }
            }
            "-m" | "--msisdn" => {
                if i + 1 < args.len() {
                    msisdn_override = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    return Err("--msisdn requires a value".into());
                }
            }
            "-h" | "--host" => {
                if i + 1 < args.len() {
                    host_override = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    return Err("--host requires a value".into());
                }
            }
            "-p" | "--port" => {
                if i + 1 < args.len() {
                    port_override = Some(args[i + 1].parse()?);
                    i += 2;
                } else {
                    return Err("--port requires a value".into());
                }
            }
            "--debug" => {
                debug_override = true;
                i += 1;
            }
            "--create-config" => {
                let default_config = UserSimulatorConfig::default();
                let config_content = toml::to_string_pretty(&default_config)?;
                fs::write(&config_path, config_content)?;
                println!("Default config created at '{}'", config_path);
                std::process::exit(0);
            }
            "--help" => {
                print_usage();
                std::process::exit(0);
            }
            _ => {
                println!("Unknown argument: {}", args[i]);
                print_usage();
                std::process::exit(1);
            }
        }
    }
    
    let mut config = load_config(&config_path)?;
    
    // Apply overrides
    if debug_override {
        config.logging.debug = true;
    }
    
    Ok((config, msisdn_override, host_override, port_override))
}

fn main() -> std::io::Result<()> {
    let (mut config, msisdn_override, host_override, port_override) = match parse_args() {
        Ok((config, msisdn, host, port)) => (config, msisdn, host, port),
        Err(e) => {
            eprintln!("Error parsing arguments: {}", e);
            print_usage();
            std::process::exit(1);
        }
    };
    
    // Apply command-line overrides
    if let Some(msisdn) = msisdn_override {
        config.phone.default_msisdn = msisdn;
    }
    if let Some(host) = host_override {
        config.server.host = host;
    }
    if let Some(port) = port_override {
        config.server.port = port;
    }
    
    if config.logging.debug {
        println!("🔧 Debug mode enabled");
        println!("📱 MSISDN: {}", config.phone.default_msisdn);
        println!("🌐 Server: {}:{}", config.server.host, config.server.port);
        println!("👤 System ID: {}", config.authentication.system_id);
        println!();
    }
    
    println!("📱 Starting USSD User Simulator...");
    println!("🏢 Operator: {}", config.phone.operator_name);
    println!("🌐 Connecting to: {}:{}", config.server.host, config.server.port);
    println!();
    
    let mut ui = UssdMobileUI::new(config);
    ui.start()?;
    
    Ok(())
}

impl UssdMobileUI {
    fn get_user_input(&self) -> std::io::Result<String> {
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        Ok(input.trim().to_string())
    }

    fn clear_screen(&self) {
        // Clear screen (works on most terminals)
        print!("\x1B[2J\x1B[1;1H");
        io::stdout().flush().unwrap();
    }
}