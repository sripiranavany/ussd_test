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

// Configuration structures
#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub server: ServerConfig,
    pub smpp: SmppConfig,
    pub ussd: UssdConfig,
    pub logging: LoggingConfig,
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
    pub service_code: String,
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
                service_code: "*123#".to_string(),
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
            logging: LoggingConfig {
                debug: false,
                log_file: "".to_string(),
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
    Terminated,
}

#[derive(Debug, Clone)]
pub struct Session {
    pub system_id: String,
    pub password: String,
    pub bound: bool,
    pub bind_type: u32,
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
}

impl UssdSmppServer {
    pub fn new(config: Config) -> Self {
        UssdSmppServer {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            ussd_sessions: Arc::new(Mutex::new(HashMap::new())),
            sequence_counter: Arc::new(Mutex::new(1)),
            config: Arc::new(config),
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
                    
                    thread::spawn(move || {
                        let mut handler = UssdConnectionHandler::new(stream, sessions, ussd_sessions, sequence_counter, config);
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
}

impl UssdConnectionHandler {
    fn new(
        stream: TcpStream,
        sessions: Arc<Mutex<HashMap<String, Session>>>,
        ussd_sessions: Arc<Mutex<HashMap<String, UssdSession>>>,
        sequence_counter: Arc<Mutex<u32>>,
        config: Arc<Config>,
    ) -> Self {
        UssdConnectionHandler {
            stream,
            sessions,
            ussd_sessions,
            sequence_counter,
            current_session: None,
            config,
        }
    }

    fn handle(&mut self) -> std::io::Result<()> {
        println!("New USSD connection established");
        
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
            let session = Session {
                system_id: system_id.clone(),
                password: password.clone(),
                bound: true,
                bind_type: pdu.header.command_id,
            };
            
            let mut sessions = self.sessions.lock().unwrap();
            sessions.insert(system_id.clone(), session);
            self.current_session = Some(system_id.clone());
            
            println!("Bind successful for system_id: {}", system_id);
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
        let message_id = self.generate_message_id();
        
        // Send SUBMIT_SM_RESP
        let response = SmppPdu {
            header: SmppHeader {
                command_length: 21,
                command_id: SUBMIT_SM_RESP,
                command_status: ESME_ROK,
                sequence_number: pdu.header.sequence_number,
            },
            body: format!("{}\0", message_id).into_bytes(),
        };
        
        self.send_pdu(response)?;
        println!("SUBMIT_SM_RESP sent with message_id: {}", message_id);
        
        // Process USSD request and send response
        self.process_ussd_request(&submit_sm)?;
        
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
            
            self.generate_ussd_response(session, &ussd_code)
        };
        
        // Send DELIVER_SM with USSD response
        thread::sleep(Duration::from_millis(100)); // Small delay to simulate processing
        self.send_ussd_response(&msisdn, &response_text)?;
        
        Ok(())
    }

    fn generate_ussd_response(&self, session: &mut UssdSession, request: &str) -> String {
        match &session.state {
            UssdState::Initial => {
                if request.starts_with(&self.config.ussd.service_code.trim_end_matches('#')) {
                    session.state = UssdState::MainMenu;
                    session.menu_level = 1;
                    format!("{}\n{}", 
                        self.config.ussd.menu.welcome_message,
                        self.config.ussd.menu.main_menu.join("\n"))
                } else {
                    session.state = UssdState::Terminated;
                    self.config.ussd.responses.invalid_code.clone()
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
            UssdState::Terminated => {
                format!("USSD session has ended. Please dial {} to start a new session.", self.config.ussd.service_code)
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
        body.push(response_text.len() as u8); // sm_length
        body.extend_from_slice(response_text.as_bytes()); // short_message

        let deliver_sm = SmppPdu {
            header: SmppHeader {
                command_length: 16 + body.len() as u32,
                command_id: DELIVER_SM,
                command_status: ESME_ROK,
                sequence_number: seq_num,
            },
            body,
        };

        self.send_pdu(deliver_sm)?;
        println!("USSD response sent to {}: {}", msisdn, response_text);
        
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
    println!("Service Code: {}", config.ussd.service_code);
    println!("System ID: {}", config.smpp.system_id);
    
    let server = UssdSmppServer::new(config);
    server.start(&addr)?;
    Ok(())
}