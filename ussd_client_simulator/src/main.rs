use std::io::{Read, Write};
use std::net::{TcpStream, TcpListener};
use std::thread;
use std::time::Duration;
use std::env;
use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};

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

pub struct UssdSmppClient {
    stream: TcpStream,
    sequence_counter: u32,
    bound: bool,
}

impl UssdSmppClient {
    pub fn new(server_addr: &str) -> std::io::Result<Self> {
        let stream = TcpStream::connect(server_addr)?;
        println!("Connected to USSD SMPP server at {}", server_addr);
        
        Ok(UssdSmppClient {
            stream,
            sequence_counter: 1,
            bound: false,
        })
    }

    pub fn bind(&mut self, system_id: &str, password: &str) -> std::io::Result<bool> {
        let mut body = Vec::new();
        body.extend_from_slice(system_id.as_bytes());
        body.push(0); // null terminator
        body.extend_from_slice(password.as_bytes());
        body.push(0); // null terminator
        body.extend_from_slice(b"USSD\0"); // system_type
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

        self.send_pdu(bind_pdu)?;
        
        // Wait for bind response
        let response = self.read_pdu()?;
        if response.header.command_id == BIND_TRANSCEIVER_RESP && response.header.command_status == ESME_ROK {
            self.bound = true;
            println!("Bind successful for system_id: {}", system_id);
            Ok(true)
        } else {
            println!("Bind failed. Status: 0x{:08x}", response.header.command_status);
            Ok(false)
        }
    }

    pub fn send_ussd_request(&mut self, from_msisdn: &str, ussd_code: &str) -> std::io::Result<String> {
        if !self.bound {
            return Err(std::io::Error::new(std::io::ErrorKind::NotConnected, "Not bound to server"));
        }

        let mut body = Vec::new();
        body.extend_from_slice(b"USSD\0"); // service_type
        body.push(1); // source_addr_ton (International)
        body.push(1); // source_addr_npi (ISDN)
        body.extend_from_slice(from_msisdn.as_bytes()); // source_addr
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
        println!("Sent USSD request from {}: {}", from_msisdn, ussd_code);

        // Wait for submit response
        let submit_resp = self.read_pdu()?;
        if submit_resp.header.command_id == SUBMIT_SM_RESP && submit_resp.header.command_status == ESME_ROK {
            let message_id = String::from_utf8_lossy(&submit_resp.body).trim_end_matches('\0').to_string();
            println!("SUBMIT_SM_RESP received, message_id: {}", message_id);
            
            // Wait for DELIVER_SM with USSD response
            let deliver_sm = self.read_pdu()?;
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
                
                Ok(response_text)
            } else {
                Err(std::io::Error::new(std::io::ErrorKind::Other, "Expected DELIVER_SM"))
            }
        } else {
            Err(std::io::Error::new(std::io::ErrorKind::Other, "SUBMIT_SM failed"))
        }
    }

    pub fn start_message_listener(&mut self) -> std::io::Result<()> {
        if !self.bound {
            return Err(std::io::Error::new(std::io::ErrorKind::NotConnected, "Not bound to server"));
        }

        println!("Starting message listener...");
        
        loop {
            match self.read_pdu() {
                Ok(pdu) => {
                    match pdu.header.command_id {
                        DELIVER_SM => {
                            let response_text = self.parse_deliver_sm(&pdu.body);
                            println!("Received USSD response: {}", response_text);
                            
                            // Send DELIVER_SM_RESP
                            let deliver_resp = SmppPdu {
                                header: SmppHeader {
                                    command_length: 16,
                                    command_id: DELIVER_SM_RESP,
                                    command_status: ESME_ROK,
                                    sequence_number: pdu.header.sequence_number,
                                },
                                body: Vec::new(),
                            };
                            self.send_pdu(deliver_resp)?;
                        }
                        ENQUIRE_LINK => {
                            // Respond to enquire_link
                            let enquire_resp = SmppPdu {
                                header: SmppHeader {
                                    command_length: 16,
                                    command_id: ENQUIRE_LINK_RESP,
                                    command_status: ESME_ROK,
                                    sequence_number: pdu.header.sequence_number,
                                },
                                body: Vec::new(),
                            };
                            self.send_pdu(enquire_resp)?;
                            println!("Responded to ENQUIRE_LINK");
                        }
                        _ => {
                            println!("Received unhandled PDU: 0x{:08x}", pdu.header.command_id);
                        }
                    }
                }
                Err(e) => {
                    println!("Error reading PDU: {}", e);
                    break;
                }
            }
        }
        
        Ok(())
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
        let response = self.read_pdu()?;
        if response.header.command_id == UNBIND_RESP {
            self.bound = false;
            println!("Unbind successful");
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

    fn get_next_sequence(&mut self) -> u32 {
        self.sequence_counter += 1;
        self.sequence_counter
    }
}

// Configuration structures
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ClientConfig {
    pub server: ServerConfig,
    pub authentication: AuthConfig,
    pub defaults: DefaultsConfig,
    pub test_cases: TestCasesConfig,
    pub logging: LoggingConfig,
    pub forwarding: Option<ForwardingConfig>, // Add forwarding configuration
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AuthConfig {
    pub system_id: String,
    pub password: String,
    pub test_system_id: String,
    pub test_password: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DefaultsConfig {
    pub default_msisdn: String,
    pub initial_ussd_code: String,
    pub request_delay_ms: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TestCasesConfig {
    pub test_cases: Vec<TestCase>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TestCase {
    pub msisdn: String,
    pub ussd_code: String,
    pub description: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LoggingConfig {
    pub debug: bool,
    pub log_file: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ForwardingConfig {
    pub listen_port: u16,
    pub enabled: bool,
    pub responses: ForwardingResponses,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ForwardingResponses {
    pub custom_services: Vec<CustomService>,
    pub menu_options: Vec<MenuOption>,
    pub default_response: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CustomService {
    pub ussd_code: String,
    pub name: String,
    pub welcome_message: String,
    pub menu_items: Vec<String>,
    pub continue_session: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MenuOption {
    pub option: String,
    pub response_text: String,
    pub continue_session: bool,
}

impl Default for ClientConfig {
    fn default() -> Self {
        ClientConfig {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 9090,
            },
            authentication: AuthConfig {
                system_id: "USSDClient".to_string(),
                password: "password123".to_string(),
                test_system_id: "USSDTestClient".to_string(),
                test_password: "testpass123".to_string(),
            },
            defaults: DefaultsConfig {
                default_msisdn: "1234567890".to_string(),
                initial_ussd_code: "*123#".to_string(),
                request_delay_ms: 500,
            },
            test_cases: TestCasesConfig {
                test_cases: vec![
                    TestCase {
                        msisdn: "1234567890".to_string(),
                        ussd_code: "*123#".to_string(),
                        description: "Test main menu access".to_string(),
                    },
                    TestCase {
                        msisdn: "1234567890".to_string(),
                        ussd_code: "1".to_string(),
                        description: "Test balance inquiry".to_string(),
                    },
                ],
            },
            logging: LoggingConfig {
                debug: false,
                log_file: "".to_string(),
            },
            forwarding: Some(ForwardingConfig {
                enabled: true,
                listen_port: 9091,
                responses: ForwardingResponses {
                    custom_services: vec![
                        CustomService {
                            ussd_code: "*100#".to_string(),
                            name: "Custom Service A".to_string(),
                            welcome_message: "Welcome to Custom Service A!".to_string(),
                            menu_items: vec![
                                "1. Check Status".to_string(),
                                "2. Get Info".to_string(),
                                "0. Exit".to_string(),
                            ],
                            continue_session: true,
                        },
                        CustomService {
                            ussd_code: "*200#".to_string(),
                            name: "Custom Service B".to_string(),
                            welcome_message: "Welcome to Custom Service B!".to_string(),
                            menu_items: vec![
                                "1. Account Details".to_string(),
                                "2. Settings".to_string(),
                                "0. Exit".to_string(),
                            ],
                            continue_session: true,
                        },
                        CustomService {
                            ussd_code: "*300#".to_string(),
                            name: "Customer Support".to_string(),
                            welcome_message: "Customer Support".to_string(),
                            menu_items: vec![
                                "Call: 1-800-HELP".to_string(),
                                "Email: support@company.com".to_string(),
                                "Thank you!".to_string(),
                            ],
                            continue_session: false,
                        },
                    ],
                    menu_options: vec![
                        MenuOption {
                            option: "1".to_string(),
                            response_text: "Status: Active\nBalance: $25.50\nNext payment: 2024-01-15\n\n0. Back to menu".to_string(),
                            continue_session: true,
                        },
                        MenuOption {
                            option: "2".to_string(),
                            response_text: "Account Details:\nName: John Doe\nPhone: +1234567890\nPlan: Premium\n\n0. Back to menu".to_string(),
                            continue_session: true,
                        },
                        MenuOption {
                            option: "0".to_string(),
                            response_text: "Thank you for using our service!".to_string(),
                            continue_session: false,
                        },
                    ],
                    default_response: "Unknown command: {}\nPlease try again or dial 0 to exit.".to_string(),
                },
            }),
        }
    }
}

impl Default for ForwardingConfig {
    fn default() -> Self {
        ForwardingConfig {
            enabled: true,
            listen_port: 9091,
            responses: ForwardingResponses {
                custom_services: vec![
                    CustomService {
                        ussd_code: "*777#".to_string(),
                        name: "Test Service".to_string(),
                        welcome_message: "Welcome to Test Service!".to_string(),
                        menu_items: vec![
                            "1. Option 1".to_string(),
                            "2. Option 2".to_string(),
                            "0. Exit".to_string(),
                        ],
                        continue_session: true,
                    },
                ],
                menu_options: vec![
                    MenuOption {
                        option: "1".to_string(),
                        response_text: "You selected option 1\n\n0. Back to menu".to_string(),
                        continue_session: true,
                    },
                    MenuOption {
                        option: "2".to_string(),
                        response_text: "You selected option 2\n\n0. Back to menu".to_string(),
                        continue_session: true,
                    },
                    MenuOption {
                        option: "0".to_string(),
                        response_text: "Thank you for using our service!".to_string(),
                        continue_session: false,
                    },
                ],
                default_response: "Unknown command: {}\nPlease try again or dial 0 to exit.".to_string(),
            },
        }
    }
}

// Simple protocol for forwarding USSD requests
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

// USSD Forwarding Service
pub struct UssdForwardingService {
    config: ClientConfig,
    listener: TcpListener,
}

impl UssdForwardingService {
    pub fn new(config: ClientConfig) -> std::io::Result<Self> {
        let forwarding_config = config.forwarding.as_ref().unwrap();
        let listener = TcpListener::bind(format!("127.0.0.1:{}", forwarding_config.listen_port))?;
        
        println!("USSD Forwarding Service listening on port {}", forwarding_config.listen_port);
        
        Ok(UssdForwardingService {
            config,
            listener,
        })
    }

    pub fn start(&self) -> std::io::Result<()> {
        for stream in self.listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    let config = self.config.clone();
                    thread::spawn(move || {
                        if let Err(e) = Self::handle_client(&mut stream, &config) {
                            eprintln!("Error handling client: {}", e);
                        }
                    });
                }
                Err(e) => {
                    eprintln!("Error accepting connection: {}", e);
                }
            }
        }
        Ok(())
    }

    fn handle_client(stream: &mut TcpStream, config: &ClientConfig) -> std::io::Result<()> {
        let mut buffer = [0; 1024];
        let bytes_read = stream.read(&mut buffer)?;
        
        if bytes_read == 0 {
            return Ok(());
        }

        let request_data = &buffer[..bytes_read];
        let request: ForwardingRequest = serde_json::from_slice(request_data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        println!("Forwarding service received request: {:?}", request);

        // Process the USSD request
        let response = Self::process_ussd_request(&request, config);
        
        // Send response back
        let response_json = serde_json::to_string(&response)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        
        stream.write_all(response_json.as_bytes())?;
        stream.flush()?;

        println!("Forwarding service sent response: {:?}", response);
        Ok(())
    }

    fn process_ussd_request(request: &ForwardingRequest, config: &ClientConfig) -> ForwardingResponse {
        let forwarding_config = config.forwarding.as_ref().unwrap();
        
        // Check if it's a custom service USSD code
        for service in &forwarding_config.responses.custom_services {
            if service.ussd_code == request.ussd_code {
                let mut response_text = service.welcome_message.clone();
                if !service.menu_items.is_empty() {
                    response_text.push('\n');
                    response_text.push_str(&service.menu_items.join("\n"));
                }
                
                return ForwardingResponse {
                    response_text,
                    continue_session: service.continue_session,
                };
            }
        }
        
        // Check if it's a menu option
        for option in &forwarding_config.responses.menu_options {
            if option.option == request.ussd_code {
                return ForwardingResponse {
                    response_text: option.response_text.clone(),
                    continue_session: option.continue_session,
                };
            }
        }
        
        // Default response for unknown commands
        ForwardingResponse {
            response_text: forwarding_config.responses.default_response.replace("{}", &request.ussd_code),
            continue_session: true,
        }
    }
}

// Interactive USSD User Simulator
pub struct UssdUserSimulator {
    client: UssdSmppClient,
    msisdn: String,
    config: ClientConfig,
}

impl UssdUserSimulator {
    pub fn new(server_addr: &str, msisdn: &str, config: ClientConfig) -> std::io::Result<Self> {
        let client = UssdSmppClient::new(server_addr)?;
        Ok(UssdUserSimulator {
            client,
            msisdn: msisdn.to_string(),
            config,
        })
    }

    pub fn start_session(&mut self) -> std::io::Result<()> {
        // Bind to server
        if !self.client.bind(&self.config.authentication.system_id, &self.config.authentication.password)? {
            return Err(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Failed to bind"));
        }

        println!("=== USSD User Simulator ===");
        println!("MSISDN: {}", self.msisdn);
        println!("Starting USSD session...");

        // Start with initial USSD code
        let mut current_input = self.config.defaults.initial_ussd_code.clone();
        
        loop {
            println!("\n--- Sending USSD Request ---");
            println!("Input: {}", current_input);
            
            match self.client.send_ussd_request(&self.msisdn, &current_input) {
                Ok(response) => {
                    println!("\n--- USSD Response ---");
                    println!("{}", response);
                    
                    if response.contains("Thank you") || response.contains("Goodbye") || response.contains("session has ended") {
                        println!("\nUSSD session terminated.");
                        break;
                    }
                    
                    // Simulate user input
                    println!("\n--- User Input Options ---");
                    println!("Enter your choice (or 'quit' to exit): ");
                    
                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input).expect("Failed to read input");
                    current_input = input.trim().to_string();
                    
                    if current_input.to_lowercase() == "quit" {
                        println!("Exiting USSD session...");
                        break;
                    }
                }
                Err(e) => {
                    println!("Error sending USSD request: {}", e);
                    break;
                }
            }
            
            // Small delay between requests
            thread::sleep(Duration::from_millis(self.config.defaults.request_delay_ms));
        }

        // Unbind from server
        self.client.unbind()?;
        println!("Disconnected from server.");
        
        Ok(())
    }
}

// Automated USSD Test Suite
pub struct UssdTestSuite {
    client: UssdSmppClient,
    config: ClientConfig,
}

impl UssdTestSuite {
    pub fn new(server_addr: &str, config: ClientConfig) -> std::io::Result<Self> {
        let client = UssdSmppClient::new(server_addr)?;
        Ok(UssdTestSuite { client, config })
    }

    pub fn run_tests(&mut self) -> std::io::Result<()> {
        println!("=== USSD Test Suite ===");
        
        // Bind to server
        if !self.client.bind(&self.config.authentication.test_system_id, &self.config.authentication.test_password)? {
            return Err(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Failed to bind"));
        }

        for test_case in &self.config.test_cases.test_cases {
            println!("\n--- Test Case: {} ---", test_case.description);
            println!("MSISDN: {}, USSD Code: {}", test_case.msisdn, test_case.ussd_code);
            
            match self.client.send_ussd_request(&test_case.msisdn, &test_case.ussd_code) {
                Ok(response) => {
                    println!("Response: {}", response);
                    println!("✓ Test passed");
                }
                Err(e) => {
                    println!("✗ Test failed: {}", e);
                }
            }
            
            thread::sleep(Duration::from_millis(1000));
        }

        // Unbind from server
        self.client.unbind()?;
        println!("\n=== All tests completed ===");
        
        Ok(())
    }
}

fn load_config(config_path: &str) -> Result<ClientConfig, Box<dyn std::error::Error>> {
    if Path::new(config_path).exists() {
        let config_content = fs::read_to_string(config_path)?;
        let config: ClientConfig = toml::from_str(&config_content)?;
        Ok(config)
    } else {
        println!("Config file not found at '{}', creating default config...", config_path);
        let default_config = ClientConfig::default();
        let config_content = toml::to_string_pretty(&default_config)?;
        fs::write(config_path, config_content)?;
        println!("Default config created at '{}'", config_path);
        Ok(default_config)
    }
}

// Function removed - usage is now printed inline

fn parse_args() -> Result<(ClientConfig, Option<String>, Option<u16>, Vec<String>), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let mut config_path = "client_config.toml".to_string();
    let mut host_override: Option<String> = None;
    let mut port_override: Option<u16> = None;
    let mut remaining_args = Vec::new();
    
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
            "--create-config" => {
                let default_config = ClientConfig::default();
                let config_content = toml::to_string_pretty(&default_config)?;
                fs::write(&config_path, config_content)?;
                println!("Default config created at '{}'", config_path);
                std::process::exit(0);
            }
            "--help" => {
                println!("Usage:");
                println!("  {} user <msisdn>     - Start interactive user simulator", std::env::args().next().unwrap_or_default());
                println!("  {} test              - Run automated test suite", std::env::args().next().unwrap_or_default());
                println!("  {} client <msisdn>   - Start basic client", std::env::args().next().unwrap_or_default());
                println!("  {} forwarding        - Start USSD forwarding service", std::env::args().next().unwrap_or_default());
                std::process::exit(0);
            }
            _ => {
                remaining_args.push(args[i].clone());
                i += 1;
            }
        }
    }
    
    let config = load_config(&config_path)?;
    Ok((config, host_override, port_override, remaining_args))
}

fn main() -> std::io::Result<()> {
    let (mut config, host_override, port_override, remaining_args) = match parse_args() {
        Ok((config, host, port, args)) => (config, host, port, args),
        Err(e) => {
            eprintln!("Error parsing arguments: {}", e);
            println!("Usage:");
            println!("  {} user <msisdn>     - Start interactive user simulator", std::env::args().next().unwrap_or_default());
            println!("  {} test              - Run automated test suite", std::env::args().next().unwrap_or_default());
            println!("  {} client <msisdn>   - Start basic client", std::env::args().next().unwrap_or_default());
            println!("  {} forwarding        - Start USSD forwarding service", std::env::args().next().unwrap_or_default());
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
    
    if remaining_args.is_empty() {
        println!("Usage:");
        println!("  {} user <msisdn>     - Start interactive user simulator", std::env::args().next().unwrap_or_default());
        println!("  {} test              - Run automated test suite", std::env::args().next().unwrap_or_default());
        println!("  {} client <msisdn>   - Start basic client", std::env::args().next().unwrap_or_default());
        println!("  {} forwarding        - Start USSD forwarding service", std::env::args().next().unwrap_or_default());
        return Ok(());
    }

    let mode = &remaining_args[0];
    let server_addr = format!("{}:{}", config.server.host, config.server.port);

    if config.logging.debug {
        println!("Debug mode enabled");
        println!("Server address: {}", server_addr);
        println!("Configuration: {:#?}", config);
    }

    match mode.as_str() {
        "user" => {
            let msisdn = remaining_args.get(1)
                .cloned()
                .unwrap_or_else(|| config.defaults.default_msisdn.clone());
            let mut user_sim = UssdUserSimulator::new(&server_addr, &msisdn, config)?;
            user_sim.start_session()?;
        }
        "test" => {
            let mut test_suite = UssdTestSuite::new(&server_addr, config)?;
            test_suite.run_tests()?;
        }
        "client" => {
            let msisdn = remaining_args.get(1)
                .cloned()
                .unwrap_or_else(|| config.defaults.default_msisdn.clone());
            let mut client = UssdSmppClient::new(&server_addr)?;
            
            if client.bind(&config.authentication.system_id, &config.authentication.password)? {
                println!("Testing basic USSD flow...");
                
                let response = client.send_ussd_request(&msisdn, &config.defaults.initial_ussd_code)?;
                println!("Response: {}", response);
                
                let response = client.send_ussd_request(&msisdn, "1")?;
                println!("Response: {}", response);
                
                client.unbind()?;
            }
        }
        "forwarding" => {
            if let Some(forwarding_config) = &config.forwarding {
                if forwarding_config.enabled {
                    println!("Starting USSD Forwarding Service...");
                    let forwarding_service = UssdForwardingService::new(config)?;
                    forwarding_service.start()?;
                } else {
                    println!("Forwarding service is disabled in configuration");
                }
            } else {
                println!("Forwarding configuration not found");
            }
        }
        _ => {
            println!("Unknown mode: {}", mode);
            println!("Usage:");
            println!("  {} user <msisdn>     - Start interactive user simulator", std::env::args().next().unwrap_or_default());
            println!("  {} test              - Run automated test suite", std::env::args().next().unwrap_or_default());
            println!("  {} client <msisdn>   - Start basic client", std::env::args().next().unwrap_or_default());
            println!("  {} forwarding        - Start USSD forwarding service", std::env::args().next().unwrap_or_default());
        }
    }

    Ok(())
}