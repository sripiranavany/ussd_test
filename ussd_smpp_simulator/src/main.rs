use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};

// Connection tracking for forwarding
#[derive(Debug, Clone)]
pub struct ConnectionManager {
    pub connections: Arc<Mutex<HashMap<String, Arc<Mutex<TcpStream>>>>>,
}

impl ConnectionManager {
    fn new() -> Self {
        ConnectionManager {
            connections: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    fn add_connection(&self, connection_id: String, stream: Arc<Mutex<TcpStream>>) {
        let mut connections = self.connections.lock().unwrap();
        connections.insert(connection_id, stream);
    }
    
    fn remove_connection(&self, connection_id: &str) {
        let mut connections = self.connections.lock().unwrap();
        connections.remove(connection_id);
    }
    
    fn get_forwarding_connection(&self, sessions: &HashMap<String, Session>) -> Option<Arc<Mutex<TcpStream>>> {
        let connections = self.connections.lock().unwrap();
        
        // Find first session that can receive forwards (custom USSD handlers) and has an active connection
        for (_, session) in sessions {
            if session.can_receive_forwards && session.bound && !session.is_user_client {
                if let Some(conn_id) = &session.connection_id {
                    if let Some(stream) = connections.get(conn_id) {
                        return Some(stream.clone());
                    }
                }
            }
        }
        None
    }
    
    fn get_user_connection(&self, sessions: &HashMap<String, Session>) -> Option<Arc<Mutex<TcpStream>>> {
        let connections = self.connections.lock().unwrap();
        
        // Find first session that is a user client and has an active connection
        for (_, session) in sessions {
            if session.is_user_client && session.bound {
                if let Some(conn_id) = &session.connection_id {
                    if let Some(stream) = connections.get(conn_id) {
                        return Some(stream.clone());
                    }
                }
            }
        }
        None
    }
}

// Configuration structures
#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub server: ServerConfig,
    pub smpp: SmppConfig,
    pub ussd: UssdConfig,
    pub client_simulator: ClientSimulatorConfig,
    pub logging: LoggingConfig,
    pub response_percentage: ResponsePercentageConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SmppConfig {
    pub system_id: String,
    pub max_connections: u32,
    pub connection_timeout: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UssdConfig {
    pub service_codes: Vec<String>,
    pub session_timeout: u64,
    pub menu: MenuConfig,
    pub responses: ResponsesConfig,
    pub data_packages: DataPackagesConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MenuConfig {
    pub welcome_message: String,
    pub main_menu: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ResponsesConfig {
    pub balance_message: String,
    pub invalid_code: String,
    pub invalid_option: String,
    pub goodbye_message: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DataPackagesConfig {
    pub packages: Vec<DataPackage>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DataPackage {
    pub name: String,
    pub price: f64,
    pub data: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LoggingConfig {
    pub debug: bool,
    pub log_file: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ClientSimulatorConfig {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub system_id: String,
    pub password: String,
    pub forwarding_clients: Vec<String>, // List of system IDs that handle custom USSD codes
    pub user_clients: Vec<String>, // List of system IDs that are user simulators
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ResponsePercentageConfig {
    pub success_percentage: f64,
    pub failure_percentage: f64,
    pub no_response_percentage: f64,
    pub failure_error_code: u32,
    pub no_response_delay_ms: u64,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 2775,
            },
            smpp: SmppConfig {
                system_id: "USSDGateway".to_string(),
                max_connections: 100,
                connection_timeout: 300,
            },
            ussd: UssdConfig {
                service_codes: vec!["*123#".to_string()],
                session_timeout: 180,
                menu: MenuConfig {
                    welcome_message: "Welcome to MyTelecom USSD Service".to_string(),
                    main_menu: vec![
                        "1. Balance Inquiry".to_string(),
                        "2. Data Packages".to_string(),
                        "3. Customer Service".to_string(),
                        "0. Exit".to_string(),
                    ],
                },
                responses: ResponsesConfig {
                    balance_message: "Your current balance is $25.50\nYour data balance is 2.5GB".to_string(),
                    invalid_code: "Invalid USSD code. Please try again.".to_string(),
                    invalid_option: "Invalid option. Please try again.".to_string(),
                    goodbye_message: "Thank you for using MyTelecom USSD Service. Goodbye!".to_string(),
                },
                data_packages: DataPackagesConfig {
                    packages: vec![
                        DataPackage {
                            name: "1GB Package".to_string(),
                            price: 10.0,
                            data: "1GB".to_string(),
                        },
                        DataPackage {
                            name: "5GB Package".to_string(),
                            price: 40.0,
                            data: "5GB".to_string(),
                        },
                        DataPackage {
                            name: "10GB Package".to_string(),
                            price: 70.0,
                            data: "10GB".to_string(),
                        },
                    ],
                },
            },
            client_simulator: ClientSimulatorConfig {
                enabled: false,
                host: "127.0.0.1".to_string(),
                port: 9090,
                system_id: "USSDClient".to_string(),
                password: "password123".to_string(),
                forwarding_clients: vec!["ForwardingClient".to_string(), "JavaClient".to_string()],
                user_clients: vec!["USSDMobileUser".to_string()],
            },
            logging: LoggingConfig {
                debug: false,
                log_file: "".to_string(),
            },
            response_percentage: ResponsePercentageConfig {
                success_percentage: 95.0,
                failure_percentage: 4.0,
                no_response_percentage: 1.0,
                failure_error_code: 0x00000008, // ESME_RSYSERR
                no_response_delay_ms: 5000,
            },
        }
    }
}

// SMPP Command IDs
const BIND_RECEIVER: u32 = 0x00000001;
const BIND_TRANSMITTER: u32 = 0x00000002;
const BIND_TRANSCEIVER: u32 = 0x00000009;
const BIND_RECEIVER_RESP: u32 = 0x80000001;
const BIND_TRANSMITTER_RESP: u32 = 0x80000002;
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
const ESME_RINVBNDSTS: u32 = 0x00000004;
const ESME_RINVPASWD: u32 = 0x0000000E;

// USSD Service Types
const USSD_NEW_REQUEST: u8 = 1;
const USSD_EXISTING_REQUEST: u8 = 2;
const USSD_TERMINATE_REQUEST: u8 = 3;
const USSD_TERMINATE_NOTIFY: u8 = 4;

// USSD Operations
const USSD_REQUEST: u8 = 1;
const USSD_NOTIFY: u8 = 2;
const USSD_RESPONSE: u8 = 3;

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
pub struct UssdSession {
    pub msisdn: String,
    pub session_id: String,
    pub state: UssdState,
    pub menu_level: u8,
    pub last_request: String,
}

#[derive(Debug, Clone)]
pub enum UssdState {
    Initial,
    MainMenu,
    BalanceInquiry,
    DataPackages,
    CustomerService,
    Forwarded,
    Terminated,
}

#[derive(Debug, Clone)]
pub struct Session {
    pub system_id: String,
    pub password: String,
    pub bound: bool,
    pub bind_type: u32,
    pub can_receive_forwards: bool,
    pub is_user_client: bool,
    pub connection_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SubmitSmPdu {
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
    pub optional_params: Vec<OptionalParam>,
}

#[derive(Debug, Clone)]
pub struct DeliverSmPdu {
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

#[derive(Debug, Clone)]
pub struct OptionalParam {
    pub tag: u16,
    pub length: u16,
    pub value: Vec<u8>,
}

pub struct UssdSmppServer {
    pub sessions: Arc<Mutex<HashMap<String, Session>>>,
    pub ussd_sessions: Arc<Mutex<HashMap<String, UssdSession>>>,
    pub sequence_counter: Arc<Mutex<u32>>,
    pub config: Arc<Config>,
    pub connection_manager: ConnectionManager,
}

impl UssdSmppServer {
    pub fn new(config: Config) -> Self {
        UssdSmppServer {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            ussd_sessions: Arc::new(Mutex::new(HashMap::new())),
            sequence_counter: Arc::new(Mutex::new(1)),
            config: Arc::new(config),
            connection_manager: ConnectionManager::new(),
        }
    }

    pub fn start(&self, addr: &str) -> std::io::Result<()> {
        let listener = TcpListener::bind(addr)?;
        println!("USSD SMPP Server listening on {}", addr);
        if self.config.logging.debug {
            println!("Debug logging enabled");
            println!("Configuration: {:#?}", self.config);
        }

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let sessions = Arc::clone(&self.sessions);
                    let ussd_sessions = Arc::clone(&self.ussd_sessions);
                    let sequence_counter = Arc::clone(&self.sequence_counter);
                    let config = Arc::clone(&self.config);
                    let connection_manager = self.connection_manager.clone();
                    
                    thread::spawn(move || {
                        let mut handler = UssdConnectionHandler::new(stream, sessions, ussd_sessions, sequence_counter, config, connection_manager);
                        if let Err(e) = handler.handle() {
                            println!("Connection error: {}", e);
                        }
                    });
                }
                Err(e) => println!("Connection failed: {}", e),
            }
        }
        Ok(())
    }
}

struct UssdConnectionHandler {
    stream: TcpStream,
    sessions: Arc<Mutex<HashMap<String, Session>>>,
    ussd_sessions: Arc<Mutex<HashMap<String, UssdSession>>>,
    sequence_counter: Arc<Mutex<u32>>,
    current_session: Option<String>,
    config: Arc<Config>,
    connection_id: String,
    connection_manager: ConnectionManager,
}

impl UssdConnectionHandler {
    fn new(
        stream: TcpStream,
        sessions: Arc<Mutex<HashMap<String, Session>>>,
        ussd_sessions: Arc<Mutex<HashMap<String, UssdSession>>>,
        sequence_counter: Arc<Mutex<u32>>,
        config: Arc<Config>,
        connection_manager: ConnectionManager,
    ) -> Self {
        // Generate unique connection ID
        let connection_id = format!("conn_{}", SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos());
        
        UssdConnectionHandler {
            stream,
            sessions,
            ussd_sessions,
            sequence_counter,
            current_session: None,
            config,
            connection_id,
            connection_manager,
        }
    }

    fn handle(&mut self) -> std::io::Result<()> {
        println!("New USSD connection established");
        
        // Add connection to manager
        self.connection_manager.add_connection(self.connection_id.clone(), Arc::new(Mutex::new(self.stream.try_clone()?)));
        
        loop {
            match self.read_pdu() {
                Ok(pdu) => {
                    if let Err(e) = self.process_pdu(pdu) {
                        println!("Error processing PDU: {}", e);
                        break;
                    }
                }
                Err(e) => {
                    println!("Error reading PDU: {}", e);
                    break;
                }
            }
        }
        
        if let Some(session_id) = &self.current_session {
            let mut sessions = self.sessions.lock().unwrap();
            sessions.remove(session_id);
            println!("Session {} disconnected", session_id);
        }
        
        // Remove connection from manager
        self.connection_manager.remove_connection(&self.connection_id);
        
        Ok(())
    }

    fn read_pdu(&mut self) -> std::io::Result<SmppPdu> {
        let mut header_buf = [0u8; 16];
        self.stream.read_exact(&mut header_buf)?;

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
            self.stream.read_exact(&mut body)?;
        }

        Ok(SmppPdu { header, body })
    }

    fn process_pdu(&mut self, pdu: SmppPdu) -> std::io::Result<()> {
        match pdu.header.command_id {
            BIND_RECEIVER | BIND_TRANSMITTER | BIND_TRANSCEIVER => {
                self.handle_bind(pdu)?;
            }
            SUBMIT_SM => {
                self.handle_ussd_submit_sm(pdu)?;
            }
            SUBMIT_SM_RESP => {
                self.handle_submit_sm_resp(pdu)?;
            }
            DELIVER_SM => {
                self.handle_deliver_sm(pdu)?;
            }
            DELIVER_SM_RESP => {
                self.handle_deliver_sm_resp(pdu)?;
            }
            ENQUIRE_LINK => {
                self.handle_enquire_link(pdu)?;
            }
            UNBIND => {
                self.handle_unbind(pdu)?;
            }
            _ => {
                println!("Unhandled command ID: 0x{:08x}", pdu.header.command_id);
            }
        }
        Ok(())
    }

    fn handle_bind(&mut self, pdu: SmppPdu) -> std::io::Result<()> {
        let (system_id, password) = self.parse_bind_request(&pdu.body);
        
        println!("Bind request from system_id: {}", system_id);
        
        let status = if !system_id.is_empty() && !password.is_empty() {
            // Check if this system_id can receive forwarded requests
            let can_receive_forwards = self.config.client_simulator.forwarding_clients
                .contains(&system_id);
            
            // Check if this is a user client
            let is_user_client = self.config.client_simulator.user_clients
                .contains(&system_id);
            
            let session = Session {
                system_id: system_id.clone(),
                password: password.clone(),
                bound: true,
                bind_type: pdu.header.command_id,
                can_receive_forwards,
                is_user_client,
                connection_id: Some(self.connection_id.clone()),
            };
            
            let mut sessions = self.sessions.lock().unwrap();
            sessions.insert(system_id.clone(), session);
            self.current_session = Some(system_id.clone());
            
            if is_user_client {
                println!("Bind successful for system_id: {} (user client)", system_id);
            } else if can_receive_forwards {
                println!("Bind successful for system_id: {} (forwarding client)", system_id);
            } else {
                println!("Bind successful for system_id: {} (regular client)", system_id);
            }
            ESME_ROK
        } else {
            println!("Bind failed for system_id: {}", system_id);
            ESME_RINVPASWD
        };

        let resp_command_id = pdu.header.command_id | 0x80000000;
        let response = self.create_bind_response(resp_command_id, status, pdu.header.sequence_number);
        self.send_pdu(response)?;
        
        Ok(())
    }

    fn handle_ussd_submit_sm(&mut self, pdu: SmppPdu) -> std::io::Result<()> {
        println!("Received USSD SUBMIT_SM");
        
        let submit_sm = self.parse_submit_sm(&pdu.body);
        
        // Determine response type based on configured percentages
        let response_type = self.determine_response_type();
        
        match response_type {
            ResponseType::Success => {
                // Normal processing - send success response
                let message_id = self.generate_message_id();
                let body = format!("{}\0", message_id).into_bytes();
                let response = SmppPdu {
                    header: SmppHeader {
                        command_length: 16 + body.len() as u32,
                        command_id: SUBMIT_SM_RESP,
                        command_status: ESME_ROK,
                        sequence_number: pdu.header.sequence_number,
                    },
                    body,
                };
                
                self.send_pdu(response)?;
                println!("SUBMIT_SM_RESP sent with message_id: {}", message_id);
                
                // Process USSD request and send response
                self.process_ussd_request(&submit_sm)?;
            }
            ResponseType::Failure => {
                // Send failure response
                println!("Simulating failure response for SUBMIT_SM");
                self.send_submit_sm_resp_error(pdu.header.sequence_number, self.config.response_percentage.failure_error_code)?;
            }
            ResponseType::NoResponse => {
                // No response - just log and delay
                println!("Simulating no response for SUBMIT_SM");
                thread::sleep(Duration::from_millis(self.config.response_percentage.no_response_delay_ms));
                // Don't send any response
            }
        }
        
        Ok(())
    }

    fn process_ussd_request(&mut self, submit_sm: &SubmitSmPdu) -> std::io::Result<()> {
        let msisdn = submit_sm.source_addr.clone();
        let ussd_code = String::from_utf8_lossy(&submit_sm.short_message).to_string();
        
        println!("Processing USSD request from {}: {}", msisdn, ussd_code);
        
        let response_text = {
            let mut ussd_sessions = self.ussd_sessions.lock().unwrap();
            let session = ussd_sessions.entry(msisdn.clone()).or_insert_with(|| {
                UssdSession {
                    msisdn: msisdn.clone(),
                    session_id: self.generate_session_id(),
                    state: UssdState::Initial,
                    menu_level: 0,
                    last_request: String::new(),
                }
            });
            
            // Check if this is a new USSD code (starts with * and ends with #) that should reset the session
            if ussd_code.starts_with('*') && ussd_code.ends_with('#') {
                session.state = UssdState::Initial;
                session.menu_level = 0;
                session.last_request = String::new();
            }
            
            self.generate_ussd_response(session, &ussd_code)
        };
        
        // Send DELIVER_SM with USSD response only if we have a response
        if !response_text.is_empty() {
            thread::sleep(Duration::from_millis(50)); // Minimal delay
            self.send_ussd_response(&msisdn, &response_text)?;
        } else {
            println!("No immediate response to send - waiting for forwarded response via DELIVER_SM");
        }
        
        Ok(())
    }

    fn generate_ussd_response(&self, session: &mut UssdSession, request: &str) -> String {
        match &session.state {
            UssdState::Initial => {
                if self.config.ussd.service_codes.iter().any(|code| request.starts_with(&code.trim_end_matches('#'))) {
                    session.state = UssdState::MainMenu;
                    session.menu_level = 1;
                    format!("{}\n{}", 
                        self.config.ussd.menu.welcome_message,
                        self.config.ussd.menu.main_menu.join("\n"))
                } else {
                    // Try to forward to bound client
                    match self.forward_to_bound_client(&session.msisdn, request) {
                        Ok(_) => {
                            session.state = UssdState::Forwarded;
                            println!("Forwarded USSD code {} to bound client", request);
                            // Return empty string - the real response will come via DELIVER_SM
                            String::new()
                        }
                        Err(e) => {
                            println!("Failed to forward USSD code {} to bound client: {}", request, e);
                            session.state = UssdState::Terminated;
                            self.config.ussd.responses.invalid_code.clone()
                        }
                    }
                }
            }
            UssdState::MainMenu => {
                match &request[..] {
                    "1" => {
                        session.state = UssdState::BalanceInquiry;
                        format!("{}\nPress 0 to return to main menu", self.config.ussd.responses.balance_message)
                    }
                    "2" => {
                        session.state = UssdState::DataPackages;
                        let mut menu = "Available Data Packages:\n".to_string();
                        for (i, package) in self.config.ussd.data_packages.packages.iter().enumerate() {
                            menu.push_str(&format!("{}. {} - ${:.2}\n", i + 1, package.data, package.price));
                        }
                        menu.push_str("0. Back to main menu");
                        menu
                    }
                    "3" => {
                        session.state = UssdState::CustomerService;
                        "Customer Service:\nCall 123 for support\nEmail: support@mytelecom.com\nPress 0 to return to main menu".to_string()
                    }
                    "0" => {
                        session.state = UssdState::Terminated;
                        self.config.ussd.responses.goodbye_message.clone()
                    }
                    _ => {
                        format!("{}\n{}", 
                            self.config.ussd.responses.invalid_option,
                            self.config.ussd.menu.main_menu.join("\n"))
                    }
                }
            }
            UssdState::BalanceInquiry | UssdState::DataPackages | UssdState::CustomerService => {
                if request == "0" {
                    session.state = UssdState::MainMenu;
                    session.menu_level = 1;
                    format!("{}\n{}", 
                        self.config.ussd.menu.welcome_message,
                        self.config.ussd.menu.main_menu.join("\n"))
                } else if request == "00" {
                    session.state = UssdState::Terminated;
                    self.config.ussd.responses.goodbye_message.clone()
                } else {
                    match &session.state {
                        UssdState::DataPackages => {
                            if let Ok(choice) = request.parse::<usize>() {
                                if choice > 0 && choice <= self.config.ussd.data_packages.packages.len() {
                                    let package = &self.config.ussd.data_packages.packages[choice - 1];
                                    format!("{} selected. Reply with 'YES' to confirm purchase for ${:.2}", 
                                        package.name, package.price)
                                } else {
                                    "Invalid option. Please select a valid package number, or 0 to go back".to_string()
                                }
                            } else if request.to_uppercase() == "YES" {
                                session.state = UssdState::MainMenu;
                                "Package purchased successfully! You will receive a confirmation SMS shortly.\nPress 0 to return to main menu".to_string()
                            } else {
                                "Invalid option. Please select a valid package number, or 0 to go back".to_string()
                            }
                        }
                        _ => "Press 0 to return to main menu or 00 to exit".to_string(),
                    }
                }
            }
            UssdState::Forwarded => {
                // Continue forwarding requests to bound client
                match self.forward_to_bound_client(&session.msisdn, request) {
                    Ok(_) => {
                        println!("Forwarded follow-up USSD request {} to bound client", request);
                        // Return empty string - the real response will come via DELIVER_SM
                        String::new()
                    }
                    Err(e) => {
                        println!("Failed to forward follow-up USSD request {} to bound client: {}", request, e);
                        session.state = UssdState::Terminated;
                        "Service temporarily unavailable. Thank you!".to_string()
                    }
                }
            }
            UssdState::Terminated => {
                let code_list = self.config.ussd.service_codes.join(", ");
                format!("USSD session has ended. Please dial one of [{}] to start a new session.", code_list)
            }
        }
    }

    fn send_ussd_response(&mut self, msisdn: &str, response_text: &str) -> std::io::Result<()> {
        let mut sequence = self.sequence_counter.lock().unwrap();
        *sequence += 1;
        let seq_num = *sequence;
        drop(sequence);

        let mut body = Vec::new();
        
        // Build DELIVER_SM PDU for USSD response
        body.extend_from_slice(b"USSD\0"); // service_type
        body.push(1); // source_addr_ton (International)
        body.push(1); // source_addr_npi (ISDN)
        body.extend_from_slice(b"123\0"); // source_addr (USSD gateway)
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
        let truncated_response = if response_text.len() > 255 {
            &response_text[..255]
        } else {
            response_text
        };
        if self.config.logging.debug {
            println!("🔤 Response text length: {} chars", truncated_response.len());
            println!("🔤 Response text: {:?}", truncated_response);
        }
        body.push(truncated_response.len() as u8); // sm_length
        body.extend_from_slice(truncated_response.as_bytes()); // short_message

        let body_len = body.len();
        let deliver_sm = SmppPdu {
            header: SmppHeader {
                command_length: 16 + body.len() as u32,
                command_id: DELIVER_SM,
                command_status: ESME_ROK,
                sequence_number: seq_num,
            },
            body,
        };

        // Send response to user simulator (not forwarding client)
        let sessions = self.sessions.lock().unwrap();
        if let Some(user_stream) = self.connection_manager.get_user_connection(&sessions) {
            println!("📤 Sending DELIVER_SM to user simulator");
            let mut stream = user_stream.lock().unwrap();
            if let Err(e) = self.send_pdu_to_stream(&mut *stream, deliver_sm) {
                println!("⚠️  Error sending to user simulator: {}", e);
                return Err(std::io::Error::new(std::io::ErrorKind::Other, e));
            }
            if self.config.logging.debug {
                println!("📦 DELIVER_SM sent to user simulator with command_id: 0x{:08x}, body_length: {}", DELIVER_SM, body_len);
            }
        } else {
            println!("⚠️  No user connection found for user simulator");
            return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "No user connection available"));
        }
        println!("USSD response sent to {}: {}", msisdn, response_text);
        
        Ok(())
    }

    fn send_submit_sm_resp_error(&mut self, sequence_number: u32, error_code: u32) -> std::io::Result<()> {
        let response = SmppPdu {
            header: SmppHeader {
                command_length: 16,
                command_id: SUBMIT_SM_RESP,
                command_status: error_code,
                sequence_number,
            },
            body: Vec::new(),
        };
        
        self.send_pdu(response)?;
        println!("SUBMIT_SM_RESP sent with error code: 0x{:08X}", error_code);
        Ok(())
    }

    fn parse_submit_sm(&self, body: &[u8]) -> SubmitSmPdu {
        let mut pos = 0;
        let service_type = self.read_c_string(body, &mut pos);
        let source_addr_ton = body[pos]; pos += 1;
        let source_addr_npi = body[pos]; pos += 1;
        let source_addr = self.read_c_string(body, &mut pos);
        let dest_addr_ton = body[pos]; pos += 1;
        let dest_addr_npi = body[pos]; pos += 1;
        let destination_addr = self.read_c_string(body, &mut pos);
        let esm_class = body[pos]; pos += 1;
        let protocol_id = body[pos]; pos += 1;
        let priority_flag = body[pos]; pos += 1;
        let schedule_delivery_time = self.read_c_string(body, &mut pos);
        let validity_period = self.read_c_string(body, &mut pos);
        let registered_delivery = body[pos]; pos += 1;
        let replace_if_present_flag = body[pos]; pos += 1;
        let data_coding = body[pos]; pos += 1;
        let sm_default_msg_id = body[pos]; pos += 1;
        let sm_length = body[pos]; pos += 1;
        let short_message = body[pos..pos + sm_length as usize].to_vec();

        SubmitSmPdu {
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
            optional_params: Vec::new(),
        }
    }

    fn parse_deliver_sm(&self, body: &[u8]) -> DeliverSmPdu {
        let mut pos = 0;
        let service_type = self.read_c_string(body, &mut pos);
        let source_addr_ton = body[pos]; pos += 1;
        let source_addr_npi = body[pos]; pos += 1;
        let source_addr = self.read_c_string(body, &mut pos);
        let dest_addr_ton = body[pos]; pos += 1;
        let dest_addr_npi = body[pos]; pos += 1;
        let destination_addr = self.read_c_string(body, &mut pos);
        let esm_class = body[pos]; pos += 1;
        let protocol_id = body[pos]; pos += 1;
        let priority_flag = body[pos]; pos += 1;
        let schedule_delivery_time = self.read_c_string(body, &mut pos);
        let validity_period = self.read_c_string(body, &mut pos);
        let registered_delivery = body[pos]; pos += 1;
        let replace_if_present_flag = body[pos]; pos += 1;
        let data_coding = body[pos]; pos += 1;
        let sm_default_msg_id = body[pos]; pos += 1;
        let sm_length = body[pos]; pos += 1;
        let short_message = body[pos..pos + sm_length as usize].to_vec();

        DeliverSmPdu {
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
        }
    }

    fn parse_bind_request(&self, body: &[u8]) -> (String, String) {
        let mut pos = 0;
        let system_id = self.read_c_string(body, &mut pos);
        let password = self.read_c_string(body, &mut pos);
        (system_id, password)
    }

    fn read_c_string(&self, data: &[u8], pos: &mut usize) -> String {
        let start = *pos;
        while *pos < data.len() && data[*pos] != 0 {
            *pos += 1;
        }
        let result = String::from_utf8_lossy(&data[start..*pos]).to_string();
        if *pos < data.len() {
            *pos += 1; // Skip null terminator
        }
        result
    }

    fn create_bind_response(&self, command_id: u32, status: u32, sequence: u32) -> SmppPdu {
        let system_id = format!("{}\0", self.config.smpp.system_id);
        let body = system_id.as_bytes().to_vec();
        
        SmppPdu {
            header: SmppHeader {
                command_length: 16 + body.len() as u32,
                command_id,
                command_status: status,
                sequence_number: sequence,
            },
            body,
        }
    }

    fn handle_deliver_sm_resp(&mut self, _pdu: SmppPdu) -> std::io::Result<()> {
        println!("Received DELIVER_SM_RESP");
        Ok(())
    }

    fn handle_submit_sm_resp(&mut self, pdu: SmppPdu) -> std::io::Result<()> {
        println!("Received SUBMIT_SM_RESP from client");
        
        if self.config.logging.debug {
            println!("📨 SUBMIT_SM_RESP: cmd=0x{:08x}, status=0x{:08x}, seq={}", 
                pdu.header.command_id, pdu.header.command_status, pdu.header.sequence_number);
        }
        
        if pdu.header.command_status == ESME_ROK {
            // Extract message_id from body if present
            if !pdu.body.is_empty() {
                let message_id = self.read_c_string(&pdu.body, &mut 0);
                println!("SUBMIT_SM_RESP received with message_id: {}", message_id);
            } else {
                println!("SUBMIT_SM_RESP received successfully");
            }
        } else {
            println!("SUBMIT_SM_RESP received with error status: 0x{:08x}", pdu.header.command_status);
        }
        
        Ok(())
    }

    fn handle_deliver_sm(&mut self, pdu: SmppPdu) -> std::io::Result<()> {
        println!("Received DELIVER_SM from client");
        
        if self.config.logging.debug {
            println!("📨 DELIVER_SM: cmd=0x{:08x}, body_len={}", 
                pdu.header.command_id, pdu.body.len());
        }
        
        // Parse the DELIVER_SM to extract the menu response
        let deliver_sm = self.parse_deliver_sm(&pdu.body);
        
        if self.config.logging.debug {
            println!("📨 DELIVER_SM parsed - source: {}, dest: {}, message: {:?}", 
                deliver_sm.source_addr, deliver_sm.destination_addr, 
                String::from_utf8_lossy(&deliver_sm.short_message));
        }
        
        // Send DELIVER_SM_RESP to acknowledge receipt from client
        let response = SmppPdu {
            header: SmppHeader {
                command_length: 16,
                command_id: DELIVER_SM_RESP,
                command_status: ESME_ROK,
                sequence_number: pdu.header.sequence_number,
            },
            body: Vec::new(),
        };
        
        self.send_pdu(response)?;
        println!("DELIVER_SM_RESP sent to client");
        
        // This DELIVER_SM contains the actual menu response from the client
        // We need to forward this response back to the user simulator
        let menu_response = String::from_utf8_lossy(&deliver_sm.short_message).to_string();
        
        println!("Received menu response from client: {}", menu_response);
        println!("Forwarding this response to user simulator via DELIVER_SM");
        
        // Send the menu response to the user simulator via DELIVER_SM
        self.send_ussd_response(&deliver_sm.destination_addr, &menu_response)?;
        
        println!("Menu response forwarded to user simulator");
        
        Ok(())
    }

    fn handle_enquire_link(&mut self, pdu: SmppPdu) -> std::io::Result<()> {
        println!("Received ENQUIRE_LINK");
        
        let response = SmppPdu {
            header: SmppHeader {
                command_length: 16,
                command_id: ENQUIRE_LINK_RESP,
                command_status: ESME_ROK,
                sequence_number: pdu.header.sequence_number,
            },
            body: Vec::new(),
        };
        
        self.send_pdu(response)?;
        Ok(())
    }

    fn handle_unbind(&mut self, pdu: SmppPdu) -> std::io::Result<()> {
        println!("Received UNBIND");
        
        let response = SmppPdu {
            header: SmppHeader {
                command_length: 16,
                command_id: UNBIND_RESP,
                command_status: ESME_ROK,
                sequence_number: pdu.header.sequence_number,
            },
            body: Vec::new(),
        };
        
        self.send_pdu(response)?;
        Ok(())
    }

    fn send_pdu(&mut self, pdu: SmppPdu) -> std::io::Result<()> {
        let mut buffer = Vec::new();
        
        buffer.extend_from_slice(&pdu.header.command_length.to_be_bytes());
        buffer.extend_from_slice(&pdu.header.command_id.to_be_bytes());
        buffer.extend_from_slice(&pdu.header.command_status.to_be_bytes());
        buffer.extend_from_slice(&pdu.header.sequence_number.to_be_bytes());
        
        buffer.extend_from_slice(&pdu.body);
        
        if self.config.logging.debug {
            println!("📤 Sending PDU: cmd=0x{:08x}, len={}, body_len={}", 
                pdu.header.command_id, pdu.header.command_length, pdu.body.len());
            if pdu.body.len() > 0 {
                println!("📤 PDU body: {:?}", pdu.body);
                println!("📤 PDU body as string: {:?}", String::from_utf8_lossy(&pdu.body));
            }
            println!("📤 Full PDU buffer ({} bytes): {:02x?}", buffer.len(), buffer);
        }
        
        self.stream.write_all(&buffer)?;
        self.stream.flush()?;
        
        Ok(())
    }

    fn generate_message_id(&self) -> String {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let mut counter = self.sequence_counter.lock().unwrap();
        *counter += 1;
        
        format!("USSD{}{:04}", timestamp, *counter)
    }

    fn generate_session_id(&self) -> String {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        format!("SESS{}", timestamp)
    }

    fn determine_response_type(&self) -> ResponseType {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        // Generate a pseudo-random number based on current time
        let mut hasher = DefaultHasher::new();
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            .hash(&mut hasher);
        
        let random_value = (hasher.finish() % 10000) as f64 / 100.0; // 0-99.99%
        
        let success_threshold = self.config.response_percentage.success_percentage;
        let failure_threshold = success_threshold + self.config.response_percentage.failure_percentage;
        
        if random_value < success_threshold {
            ResponseType::Success
        } else if random_value < failure_threshold {
            ResponseType::Failure
        } else {
            ResponseType::NoResponse
        }
    }

    fn get_next_sequence(&self) -> u32 {
        let mut counter = self.sequence_counter.lock().unwrap();
        *counter += 1;
        *counter
    }
}

#[derive(Debug, Clone)]
pub enum ResponseType {
    Success,
    Failure,
    NoResponse,
}

// Forwarding structures for communication with client simulator
#[derive(Debug, Deserialize, Serialize)]
pub struct ForwardingRequest {
    pub msisdn: String,
    pub ussd_code: String,
    pub session_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ForwardingResponse {
    pub response_text: String,
    pub continue_session: bool,
}

fn load_config(config_path: &str) -> Result<Config, Box<dyn std::error::Error>> {
    if Path::new(config_path).exists() {
        let config_content = fs::read_to_string(config_path)?;
        let config: Config = toml::from_str(&config_content)?;
        Ok(config)
    } else {
        println!("Config file not found at '{}', creating default config...", config_path);
        let default_config = Config::default();
        let config_content = toml::to_string_pretty(&default_config)?;
        fs::write(config_path, config_content)?;
        println!("Default config created at '{}'", config_path);
        Ok(default_config)
    }
}

fn print_usage() {
    println!("USSD SMPP Simulator");
    println!("Usage: ussd_smpp_simulator [OPTIONS]");
    println!();
    println!("Options:");
    println!("  -c, --config <CONFIG>    Path to configuration file (default: config.toml)");
    println!("  -h, --host <HOST>        Override host from config");
    println!("  -p, --port <PORT>        Override port from config");
    println!("  --create-config          Create a default config file and exit");
    println!("  --help                   Show this help message");
    println!();
    println!("Examples:");
    println!("  ussd_smpp_simulator");
    println!("  ussd_smpp_simulator -c /path/to/config.toml");
    println!("  ussd_smpp_simulator --config myconfig.toml --host 0.0.0.0");
    println!("  ussd_smpp_simulator --create-config");
}

fn parse_args() -> Result<(Config, Option<String>, Option<u16>), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let mut config_path = "config.toml".to_string();
    let mut host_override: Option<String> = None;
    let mut port_override: Option<u16> = None;
    
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-c" | "--config" => {
                if i + 1 < args.len() {
                    config_path = args[i + 1].clone();
                    i += 2;
                } else {
                    eprintln!("Error: Config argument requires a value");
                    print_usage();
                    std::process::exit(1);
                }
            }
            "-h" | "--host" => {
                if i + 1 < args.len() {
                    host_override = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("Error: Host argument requires a value");
                    print_usage();
                    std::process::exit(1);
                }
            }
            "-p" | "--port" => {
                if i + 1 < args.len() {
                    match args[i + 1].parse::<u16>() {
                        Ok(p) => {
                            port_override = Some(p);
                            i += 2;
                        }
                        Err(_) => {
                            eprintln!("Error: Invalid port number '{}'", args[i + 1]);
                            print_usage();
                            std::process::exit(1);
                        }
                    }
                } else {
                    eprintln!("Error: Port argument requires a value");
                    print_usage();
                    std::process::exit(1);
                }
            }
            "--create-config" => {
                let default_config = Config::default();
                let config_content = toml::to_string_pretty(&default_config)?;
                fs::write("config.toml", config_content)?;
                println!("Default configuration file created: config.toml");
                println!("Edit this file to customize your USSD SMPP simulator settings.");
                std::process::exit(0);
            }
            "--help" => {
                print_usage();
                std::process::exit(0);
            }
            _ => {
                eprintln!("Error: Unknown argument '{}'", args[i]);
                print_usage();
                std::process::exit(1);
            }
        }
    }
    
    let config = load_config(&config_path)?;
    Ok((config, host_override, port_override))
}

// Function to forward USSD requests to client simulator
fn forward_ussd_request(config: &Config, msisdn: &str, ussd_code: &str) -> Result<String, String> {
    let client_config = &config.client_simulator;
    
    if !client_config.enabled {
        return Err("Client simulator forwarding is disabled".to_string());
    }
    
    let server_addr = format!("{}:{}", client_config.host, client_config.port);
    
    // Create forwarding request
    let request = ForwardingRequest {
        msisdn: msisdn.to_string(),
        ussd_code: ussd_code.to_string(),
        session_id: None,
    };
    
    // Connect to client simulator
    match TcpStream::connect(&server_addr) {
        Ok(mut stream) => {
            // Send request
            let request_json = serde_json::to_string(&request)
                .map_err(|e| format!("Failed to serialize request: {}", e))?;
            
            stream.write_all(request_json.as_bytes())
                .map_err(|e| format!("Failed to send request: {}", e))?;
            
            stream.flush()
                .map_err(|e| format!("Failed to flush stream: {}", e))?;
            
            // Read response
            let mut buffer = [0; 1024];
            let bytes_read = stream.read(&mut buffer)
                .map_err(|e| format!("Failed to read response: {}", e))?;
            
            let response_data = &buffer[..bytes_read];
            let response: ForwardingResponse = serde_json::from_slice(response_data)
                .map_err(|e| format!("Failed to parse response: {}", e))?;
            
            println!("Forwarded USSD request {} from {} to client simulator, got response: {}", 
                     ussd_code, msisdn, response.response_text);
            
            Ok(response.response_text)
        }
        Err(e) => Err(format!("Failed to connect to client simulator at {}: {}", server_addr, e))
    }
}

impl UssdConnectionHandler {
    fn forward_to_bound_client(&self, msisdn: &str, ussd_code: &str) -> Result<String, String> {
        let sessions = self.sessions.lock().unwrap();
        
        // Find a bound client that can receive forwards
        if let Some(forward_stream) = self.connection_manager.get_forwarding_connection(&sessions) {
            // Create a SUBMIT_SM to forward the request
            let submit_sm = self.create_forward_submit_sm(msisdn, ussd_code)?;
            
            // Send via SMPP
            {
                let mut stream = forward_stream.lock().unwrap();
                self.send_pdu_to_stream(&mut *stream, submit_sm)?;
            }
            
            println!("Forwarded USSD request {} to bound client", ussd_code);
            
            // Return empty string - the real response will come via DELIVER_SM
            Ok(String::new())
        } else {
            Err("No bound forwarding client available".to_string())
        }
    }
    
    fn create_forward_submit_sm(&self, msisdn: &str, ussd_code: &str) -> Result<SmppPdu, String> {
        let mut body = Vec::new();
        
        // Build SUBMIT_SM PDU for forwarding
        body.extend_from_slice(b"USSD\0"); // service_type
        body.push(1); // source_addr_ton
        body.push(1); // source_addr_npi
        body.extend_from_slice(msisdn.as_bytes());
        body.push(0); // null terminator
        body.push(0); // dest_addr_ton
        body.push(0); // dest_addr_npi
        body.extend_from_slice(b"FORWARD\0"); // destination_addr
        body.push(0x40); // esm_class (USSD)
        body.push(0); // protocol_id
        body.push(0); // priority_flag
        body.extend_from_slice(b"\0"); // schedule_delivery_time
        body.extend_from_slice(b"\0"); // validity_period
        body.push(0); // registered_delivery
        body.push(0); // replace_if_present_flag
        body.push(0); // data_coding
        body.push(0); // sm_default_msg_id
        body.push(ussd_code.len() as u8); // sm_length
        body.extend_from_slice(ussd_code.as_bytes());
        
        Ok(SmppPdu {
            header: SmppHeader {
                command_length: 16 + body.len() as u32,
                command_id: SUBMIT_SM,
                command_status: ESME_ROK,
                sequence_number: self.get_next_sequence(),
            },
            body,
        })
    }
    
    fn send_pdu_to_stream(&self, stream: &mut TcpStream, pdu: SmppPdu) -> Result<(), String> {
        let mut data = Vec::new();
        
        // Write header
        data.extend_from_slice(&pdu.header.command_length.to_be_bytes());
        data.extend_from_slice(&pdu.header.command_id.to_be_bytes());
        data.extend_from_slice(&pdu.header.command_status.to_be_bytes());
        data.extend_from_slice(&pdu.header.sequence_number.to_be_bytes());
        
        // Write body
        data.extend_from_slice(&pdu.body);
        
        stream.write_all(&data).map_err(|e| e.to_string())?;
        stream.flush().map_err(|e| e.to_string())?;
        
        Ok(())
    }
}

fn main() -> std::io::Result<()> {
    let (mut config, host_override, port_override) = match parse_args() {
        Ok((config, host, port)) => (config, host, port),
        Err(e) => {
            eprintln!("Error loading configuration: {}", e);
            std::process::exit(1);
        }
    };
    
    // Apply command-line overrides
    if let Some(host) = host_override {
        config.server.host = host;
    }
    if let Some(port) = port_override {
        config.server.port = port;
    }
    
    let addr = format!("{}:{}", config.server.host, config.server.port);
    
    println!("Starting USSD SMPP Simulator");
    println!("Service Codes: {:?}", config.ussd.service_codes);
    println!("System ID: {}", config.smpp.system_id);
    
    let server = UssdSmppServer::new(config);
    server.start(&addr)?;
    Ok(())
}