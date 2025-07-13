use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Result, anyhow};
use clap::{Arg, Command};
use log::{info, debug, error, warn};

mod config;
mod smpp;
mod ussd;

use config::ClientConfig;
use smpp::{SmppClient, SmppPdu, SmppHeader};
use ussd::{UssdMenuManager, UssdSession};

// SMPP Command IDs
const BIND_TRANSCEIVER: u32 = 0x00000009;
const BIND_TRANSCEIVER_RESP: u32 = 0x80000009;
const SUBMIT_SM: u32 = 0x00000004;
const SUBMIT_SM_RESP: u32 = 0x80000004;
const DELIVER_SM: u32 = 0x00000005;
const DELIVER_SM_RESP: u32 = 0x80000005;
const UNBIND: u32 = 0x00000006;
const UNBIND_RESP: u32 = 0x80000006;
const ENQUIRE_LINK: u32 = 0x00000015;
const ENQUIRE_LINK_RESP: u32 = 0x80000015;

// SMPP Status Codes
const ESME_ROK: u32 = 0x00000000;

#[derive(Debug, Clone)]
pub struct ForwardingClientApp {
    config: ClientConfig,
    smpp_client: Arc<Mutex<Option<SmppClient>>>,
    menu_manager: Arc<UssdMenuManager>,
    sessions: Arc<Mutex<HashMap<String, UssdSession>>>,
    sequence_counter: Arc<Mutex<u32>>,
    running: Arc<Mutex<bool>>,
}

impl ForwardingClientApp {
    pub fn new(config: ClientConfig) -> Self {
        let menu_manager = Arc::new(UssdMenuManager::new(config.clone()));
        
        ForwardingClientApp {
            config,
            smpp_client: Arc::new(Mutex::new(None)),
            menu_manager,
            sessions: Arc::new(Mutex::new(HashMap::new())),
            sequence_counter: Arc::new(Mutex::new(1)),
            running: Arc::new(Mutex::new(false)),
        }
    }

    pub async fn start(&self) -> Result<()> {
        info!("🚀 Starting USSD SMPP Client Simulator");
        info!("📡 Connecting to server: {}:{}", self.config.client.host, self.config.client.port);
        info!("🆔 System ID: {}", self.config.client.system_id);

        // Set running state
        *self.running.lock().unwrap() = true;

        // Connect and bind to SMPP server
        self.connect_and_bind().await?;

        // Start message processing loop
        self.start_message_loop().await?;

        Ok(())
    }

    async fn connect_and_bind(&self) -> Result<()> {
        let mut client = SmppClient::new(
            &self.config.client.host,
            self.config.client.port,
            &self.config.client.system_id,
            &self.config.client.password,
        );

        client.connect().await?;
        client.bind().await?;

        *self.smpp_client.lock().unwrap() = Some(client);
        info!("✅ Successfully connected and bound to SMPP server");

        Ok(())
    }

    async fn start_message_loop(&self) -> Result<()> {
        info!("👂 Starting message processing loop");

        while *self.running.lock().unwrap() {
            // Extract client temporarily to avoid holding lock during async operations
            let client_option = {
                let mut client_guard = self.smpp_client.lock().unwrap();
                client_guard.take()
            };

            if let Some(mut client) = client_option {
                match client.read_pdu().await {
                    Ok(pdu) => {
                        // Put client back before processing PDU
                        *self.smpp_client.lock().unwrap() = Some(client);
                        
                        if let Err(e) = self.process_pdu(pdu).await {
                            error!("❌ Error processing PDU: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("❌ Error reading PDU: {}", e);
                        if self.config.client.auto_reconnect {
                            warn!("🔄 Attempting to reconnect...");
                            if let Err(e) = self.connect_and_bind().await {
                                error!("❌ Reconnection failed: {}", e);
                                thread::sleep(Duration::from_secs(5));
                            }
                        } else {
                            break;
                        }
                    }
                }
            } else {
                // No client available, small delay
                tokio::time::sleep(Duration::from_millis(100)).await;
            }

            // Small delay to prevent busy waiting
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        info!("🛑 Message processing loop stopped");
        Ok(())
    }

    async fn process_pdu(&self, pdu: SmppPdu) -> Result<()> {
        debug!("📥 Received PDU: cmd=0x{:08x}, seq={}", pdu.header.command_id, pdu.header.sequence_number);

        match pdu.header.command_id {
            SUBMIT_SM => {
                self.handle_submit_sm(pdu).await?;
            }
            DELIVER_SM_RESP => {
                self.handle_deliver_sm_resp(pdu).await?;
            }
            ENQUIRE_LINK => {
                self.handle_enquire_link(pdu).await?;
            }
            UNBIND => {
                self.handle_unbind(pdu).await?;
            }
            _ => {
                warn!("🤷 Unhandled command ID: 0x{:08x}", pdu.header.command_id);
            }
        }

        Ok(())
    }

    async fn handle_submit_sm(&self, pdu: SmppPdu) -> Result<()> {
        info!("📨 Received SUBMIT_SM (forwarded USSD request)");

        // Parse the SUBMIT_SM to extract USSD information
        let submit_sm = self.parse_submit_sm(&pdu.body)?;
        let ussd_code = String::from_utf8_lossy(&submit_sm.short_message);
        let msisdn = submit_sm.source_addr.clone();

        info!("🔄 Processing forwarded USSD request: {} from {}", ussd_code, msisdn);

        // Send SUBMIT_SM_RESP first
        debug!("📤 Sending SUBMIT_SM_RESP...");
        self.send_submit_sm_resp(pdu.header.sequence_number).await?;

        // Process the USSD code and generate response with timeout
        debug!("🔄 Processing USSD request...");
        let response = tokio::time::timeout(
            Duration::from_secs(10), // 10 second timeout
            self.process_ussd_request(&msisdn, &ussd_code)
        ).await;

        let response = match response {
            Ok(Ok(response)) => response,
            Ok(Err(e)) => {
                error!("❌ Error processing USSD request: {}", e);
                "🔧 System temporarily unavailable. Please try again later.".to_string()
            }
            Err(_) => {
                error!("⏰ USSD processing timed out");
                "⏰ Request timed out. Please try again.".to_string()
            }
        };

        // Send response back via DELIVER_SM
        debug!("📤 Sending DELIVER_SM response...");
        self.send_deliver_sm(&msisdn, &response).await?;

        debug!("✅ SUBMIT_SM handling completed successfully");
        Ok(())
    }

    async fn process_ussd_request(&self, msisdn: &str, ussd_code: &str) -> Result<String> {
        debug!("🔍 Processing USSD request: {} from {}", ussd_code, msisdn);
        
        debug!("🔒 Acquiring sessions lock...");
        let mut sessions = self.sessions.lock().unwrap();
        debug!("✅ Sessions lock acquired");
        
        // Get or create session
        let session = sessions.entry(msisdn.to_string()).or_insert_with(|| {
            debug!("📝 Creating new session for {}", msisdn);
            UssdSession::new(msisdn.to_string())
        });

        debug!("📋 Current session state: menu={}, depth={}", session.current_menu, session.menu_depth);

        // Process the USSD code through the menu manager
        debug!("🔄 Calling menu_manager.process_input...");
        let response = self.menu_manager.process_input(session, ussd_code);
        debug!("✅ Menu manager returned response");

        debug!("📤 Generated response: {}", response);

        // Update session state
        debug!("🔄 Updating session last activity...");
        session.update_last_activity();
        debug!("✅ Session updated");

        debug!("🔓 Releasing sessions lock...");
        drop(sessions);
        debug!("✅ Sessions lock released");

        debug!("✅ USSD processing completed successfully");
        Ok(response)
    }

    async fn send_submit_sm_resp(&self, sequence_number: u32) -> Result<()> {
        debug!("🔄 Generating message ID...");
        let message_id = self.generate_message_id();
        debug!("✅ Generated message ID: {}", message_id);
        
        debug!("🔄 Building SUBMIT_SM_RESP body...");
        let body = format!("{}\0", message_id).into_bytes();
        debug!("✅ Body built, length: {}", body.len());

        debug!("🔄 Creating SUBMIT_SM_RESP PDU...");
        let response = SmppPdu {
            header: SmppHeader {
                command_length: 16 + body.len() as u32,
                command_id: SUBMIT_SM_RESP,
                command_status: ESME_ROK,
                sequence_number,
            },
            body,
        };
        debug!("✅ PDU created");

        debug!("🔒 Acquiring SMPP client lock...");
        let mut client_guard = self.smpp_client.lock().unwrap();
        if let Some(client) = client_guard.as_mut() {
            debug!("✅ SMPP client lock acquired");
            debug!("📤 Sending PDU...");
            client.send_pdu(response).await?;
            debug!("✅ PDU sent successfully");
            info!("📤 Sent SUBMIT_SM_RESP with message_id: {}", message_id);
        } else {
            debug!("❌ No SMPP client available");
            return Err(anyhow!("No SMPP client available"));
        }
        // Lock is automatically released when client_guard goes out of scope

        debug!("✅ SUBMIT_SM_RESP sending completed");
        Ok(())
    }

    async fn send_deliver_sm(&self, msisdn: &str, response_text: &str) -> Result<()> {
        debug!("🔄 Building DELIVER_SM PDU...");
        let mut sequence = self.sequence_counter.lock().unwrap();
        *sequence += 1;
        let seq_num = *sequence;
        drop(sequence);

        let mut body = Vec::new();
        
        // Build DELIVER_SM PDU
        body.extend_from_slice(b"USSD\0"); // service_type
        body.push(1); // source_addr_ton
        body.push(1); // source_addr_npi
        body.extend_from_slice(b"FORWARD\0"); // source_addr (forwarding client)
        body.push(1); // dest_addr_ton
        body.push(1); // dest_addr_npi
        body.extend_from_slice(msisdn.as_bytes()); // destination_addr
        body.push(0); // null terminator
        body.push(0x40); // esm_class (USSD indication)
        body.push(0); // protocol_id
        body.push(0); // priority_flag
        body.extend_from_slice(b"\0"); // schedule_delivery_time
        body.extend_from_slice(b"\0"); // validity_period
        body.push(0); // registered_delivery
        body.push(0); // replace_if_present_flag
        body.push(0); // data_coding (GSM 7-bit)
        body.push(0); // sm_default_msg_id
        
        // Truncate response if too long
        let truncated_response = if response_text.len() > 255 {
            &response_text[..255]
        } else {
            response_text
        };
        
        body.push(truncated_response.len() as u8); // sm_length
        body.extend_from_slice(truncated_response.as_bytes()); // short_message

        let deliver_sm = SmppPdu {
            header: SmppHeader {
                command_length: 16 + body.len() as u32,
                command_id: DELIVER_SM,
                command_status: ESME_ROK,
                sequence_number: seq_num,
            },
            body,
        };

        debug!("🔒 Acquiring SMPP client lock for DELIVER_SM...");
        let mut client_guard = self.smpp_client.lock().unwrap();
        if let Some(client) = client_guard.as_mut() {
            debug!("✅ SMPP client lock acquired for DELIVER_SM");
            client.send_pdu(deliver_sm).await?;
            debug!("✅ DELIVER_SM sent successfully");
            info!("📤 Sent DELIVER_SM response to {}: {}", msisdn, truncated_response);
        } else {
            return Err(anyhow!("No SMPP client available for DELIVER_SM"));
        }
        // Lock is automatically released when client_guard goes out of scope

        Ok(())
    }

    async fn handle_deliver_sm_resp(&self, _pdu: SmppPdu) -> Result<()> {
        debug!("📥 Received DELIVER_SM_RESP");
        Ok(())
    }

    async fn handle_enquire_link(&self, pdu: SmppPdu) -> Result<()> {
        debug!("💓 Received ENQUIRE_LINK");

        let response = SmppPdu {
            header: SmppHeader {
                command_length: 16,
                command_id: ENQUIRE_LINK_RESP,
                command_status: ESME_ROK,
                sequence_number: pdu.header.sequence_number,
            },
            body: Vec::new(),
        };

        if let Some(client) = self.smpp_client.lock().unwrap().as_mut() {
            client.send_pdu(response).await?;
        }

        Ok(())
    }

    async fn handle_unbind(&self, pdu: SmppPdu) -> Result<()> {
        info!("📴 Received UNBIND request");

        let response = SmppPdu {
            header: SmppHeader {
                command_length: 16,
                command_id: UNBIND_RESP,
                command_status: ESME_ROK,
                sequence_number: pdu.header.sequence_number,
            },
            body: Vec::new(),
        };

        if let Some(client) = self.smpp_client.lock().unwrap().as_mut() {
            client.send_pdu(response).await?;
        }

        *self.running.lock().unwrap() = false;
        Ok(())
    }

    fn parse_submit_sm(&self, body: &[u8]) -> Result<SubmitSm> {
        let mut pos = 0;
        
        let service_type = self.read_c_string(body, &mut pos)?;
        let source_addr_ton = self.read_byte(body, &mut pos)?;
        let source_addr_npi = self.read_byte(body, &mut pos)?;
        let source_addr = self.read_c_string(body, &mut pos)?;
        let dest_addr_ton = self.read_byte(body, &mut pos)?;
        let dest_addr_npi = self.read_byte(body, &mut pos)?;
        let destination_addr = self.read_c_string(body, &mut pos)?;
        let esm_class = self.read_byte(body, &mut pos)?;
        let protocol_id = self.read_byte(body, &mut pos)?;
        let priority_flag = self.read_byte(body, &mut pos)?;
        let schedule_delivery_time = self.read_c_string(body, &mut pos)?;
        let validity_period = self.read_c_string(body, &mut pos)?;
        let registered_delivery = self.read_byte(body, &mut pos)?;
        let replace_if_present_flag = self.read_byte(body, &mut pos)?;
        let data_coding = self.read_byte(body, &mut pos)?;
        let sm_default_msg_id = self.read_byte(body, &mut pos)?;
        let sm_length = self.read_byte(body, &mut pos)?;
        
        let short_message = if pos + sm_length as usize <= body.len() {
            body[pos..pos + sm_length as usize].to_vec()
        } else {
            Vec::new()
        };

        Ok(SubmitSm {
            service_type,
            source_addr_ton,
            source_addr_npi,
            source_addr,
            dest_addr_ton,
            dest_addr_npi,
            destination_addr,
            esm_class,
            protocol_id,
            priority_flag,
            schedule_delivery_time,
            validity_period,
            registered_delivery,
            replace_if_present_flag,
            data_coding,
            sm_default_msg_id,
            sm_length,
            short_message,
        })
    }

    fn read_c_string(&self, data: &[u8], pos: &mut usize) -> Result<String> {
        let start = *pos;
        while *pos < data.len() && data[*pos] != 0 {
            *pos += 1;
        }
        let result = String::from_utf8_lossy(&data[start..*pos]).to_string();
        if *pos < data.len() {
            *pos += 1; // Skip null terminator
        }
        Ok(result)
    }

    fn read_byte(&self, data: &[u8], pos: &mut usize) -> Result<u8> {
        if *pos >= data.len() {
            return Err(anyhow!("Unexpected end of data"));
        }
        let result = data[*pos];
        *pos += 1;
        Ok(result)
    }

    fn generate_message_id(&self) -> String {
        debug!("🔄 Getting timestamp...");
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        debug!("✅ Timestamp: {}", timestamp);
        
        debug!("🔒 Acquiring sequence counter lock...");
        let mut counter = self.sequence_counter.lock().unwrap();
        *counter += 1;
        let current_counter = *counter;
        debug!("✅ Counter incremented to: {}", current_counter);
        
        let message_id = format!("FCLIENT{}{:04}", timestamp, current_counter);
        debug!("✅ Generated message ID: {}", message_id);
        message_id
    }

    pub async fn stop(&self) -> Result<()> {
        info!("🛑 Stopping USSD SMPP Client Simulator");
        *self.running.lock().unwrap() = false;

        // Extract client from the mutex and disconnect
        let client = self.smpp_client.lock().unwrap().take();
        if let Some(mut client) = client {
            client.disconnect().await?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SubmitSm {
    pub service_type: String,
    pub source_addr_ton: u8,
    pub source_addr_npi: u8,
    pub source_addr: String,
    pub dest_addr_ton: u8,
    pub dest_addr_npi: u8,
    pub destination_addr: String,
    pub esm_class: u8,
    pub protocol_id: u8,
    pub priority_flag: u8,
    pub schedule_delivery_time: String,
    pub validity_period: String,
    pub registered_delivery: u8,
    pub replace_if_present_flag: u8,
    pub data_coding: u8,
    pub sm_default_msg_id: u8,
    pub sm_length: u8,
    pub short_message: Vec<u8>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments first
    let matches = Command::new("USSD SMPP Client Simulator")
        .version("1.0")
        .author("Your Name <your.email@example.com>")
        .about("A configurable USSD SMPP client simulator for handling custom USSD codes")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Configuration file path")
                .default_value("client_config.toml")
        )
        .arg(
            Arg::new("debug")
                .short('d')
                .long("debug")
                .help("Enable debug logging")
                .action(clap::ArgAction::SetTrue)
        )
        .get_matches();

    let config_path = matches.get_one::<String>("config").unwrap();
    let debug = matches.get_flag("debug");

    // Load configuration
    let mut config = ClientConfig::load(config_path)?;
    
    // Override debug setting from command line
    if debug {
        config.logging.debug = true;
    }

    // Initialize logging based on configuration
    let log_level = if config.logging.debug {
        "debug"
    } else {
        &config.logging.level
    };
    
    env_logger::Builder::from_default_env()
        .filter_level(match log_level {
            "trace" => log::LevelFilter::Trace,
            "debug" => log::LevelFilter::Debug,
            "info" => log::LevelFilter::Info,
            "warn" => log::LevelFilter::Warn,
            "error" => log::LevelFilter::Error,
            _ => log::LevelFilter::Info,
        })
        .init();

    info!("🚀 Starting USSD SMPP Client Simulator");
    info!("📄 Using config file: {}", config_path);
    info!("📊 Log level: {}", log_level);

    // Create and start the application
    let app = ForwardingClientApp::new(config);
    
    // Set up signal handling for graceful shutdown
    let app_clone = app.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.expect("Failed to listen for ctrl-c");
        info!("🛑 Received shutdown signal");
        if let Err(e) = app_clone.stop().await {
            error!("❌ Error during shutdown: {}", e);
        }
    });

    // Start the application
    app.start().await?;

    Ok(())
}
