use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use anyhow::{Result, anyhow};
use log::{debug, info, error};

// SMPP Command IDs
const BIND_TRANSCEIVER: u32 = 0x00000009;
const BIND_TRANSCEIVER_RESP: u32 = 0x80000009;
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

#[derive(Debug)]
pub struct SmppClient {
    host: String,
    port: u16,
    system_id: String,
    password: String,
    stream: Option<TcpStream>,
    sequence_counter: u32,
    bound: bool,
}

impl SmppClient {
    pub fn new(host: &str, port: u16, system_id: &str, password: &str) -> Self {
        SmppClient {
            host: host.to_string(),
            port,
            system_id: system_id.to_string(),
            password: password.to_string(),
            stream: None,
            sequence_counter: 1,
            bound: false,
        }
    }

    pub async fn connect(&mut self) -> Result<()> {
        info!("🔌 Connecting to SMPP server at {}:{}", self.host, self.port);
        
        let stream = TcpStream::connect(format!("{}:{}", self.host, self.port)).await?;
        
        self.stream = Some(stream);
        info!("✅ Connected to SMPP server");
        
        Ok(())
    }

    pub async fn bind(&mut self) -> Result<()> {
        if self.stream.is_none() {
            return Err(anyhow!("Not connected to server"));
        }

        info!("🔗 Binding to SMPP server as {}", self.system_id);

        // Create bind request
        let mut body = Vec::new();
        body.extend_from_slice(self.system_id.as_bytes());
        body.push(0); // null terminator
        body.extend_from_slice(self.password.as_bytes());
        body.push(0); // null terminator
        body.extend_from_slice(b"SMPP\0"); // system_type
        body.push(0x34); // interface_version
        body.push(0x00); // addr_ton
        body.push(0x00); // addr_npi
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

        // Send bind request
        self.send_pdu(bind_pdu).await?;

        // Read bind response
        let response = self.read_pdu().await?;
        
        if response.header.command_id == BIND_TRANSCEIVER_RESP && response.header.command_status == ESME_ROK {
            self.bound = true;
            info!("✅ Successfully bound to SMPP server");
            Ok(())
        } else {
            Err(anyhow!("Bind failed with status: 0x{:08x}", response.header.command_status))
        }
    }

    pub async fn send_pdu(&mut self, pdu: SmppPdu) -> Result<()> {
        if let Some(stream) = &mut self.stream {
            let mut buffer = Vec::new();
            
            // Write header
            buffer.extend_from_slice(&pdu.header.command_length.to_be_bytes());
            buffer.extend_from_slice(&pdu.header.command_id.to_be_bytes());
            buffer.extend_from_slice(&pdu.header.command_status.to_be_bytes());
            buffer.extend_from_slice(&pdu.header.sequence_number.to_be_bytes());
            
            // Write body
            buffer.extend_from_slice(&pdu.body);
            
            debug!("📤 Sending PDU: cmd=0x{:08x}, seq={}, len={}", 
                pdu.header.command_id, pdu.header.sequence_number, buffer.len());
            
            stream.write_all(&buffer).await?;
            stream.flush().await?;
            
            Ok(())
        } else {
            Err(anyhow!("Not connected to server"))
        }
    }

    pub async fn read_pdu(&mut self) -> Result<SmppPdu> {
        if let Some(stream) = &mut self.stream {
            // Read header
            let mut header_buf = [0u8; 16];
            stream.read_exact(&mut header_buf).await?;

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

            // Read body
            let body_length = command_length.saturating_sub(16) as usize;
            let mut body = vec![0u8; body_length];
            if body_length > 0 {
                stream.read_exact(&mut body).await?;
            }

            debug!("📥 Received PDU: cmd=0x{:08x}, seq={}, status=0x{:08x}", 
                command_id, sequence_number, command_status);

            Ok(SmppPdu { header, body })
        } else {
            Err(anyhow!("Not connected to server"))
        }
    }

    pub async fn disconnect(&mut self) -> Result<()> {
        if self.bound {
            info!("📴 Disconnecting from SMPP server");
            
            // Send unbind request
            let unbind_pdu = SmppPdu {
                header: SmppHeader {
                    command_length: 16,
                    command_id: UNBIND,
                    command_status: ESME_ROK,
                    sequence_number: self.get_next_sequence(),
                },
                body: Vec::new(),
            };

            if let Err(e) = self.send_pdu(unbind_pdu).await {
                error!("❌ Error sending unbind: {}", e);
            }

            self.bound = false;
        }

        if let Some(stream) = self.stream.take() {
            drop(stream);
        }

        info!("✅ Disconnected from SMPP server");
        Ok(())
    }

    fn get_next_sequence(&mut self) -> u32 {
        self.sequence_counter += 1;
        self.sequence_counter
    }

    pub fn is_bound(&self) -> bool {
        self.bound
    }
}
