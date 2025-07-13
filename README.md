# USSD SMPP Simulator System

A comprehensive USSD (Unstructured Supplementary Service Data) system simulator built in Rust that demonstrates the interaction between mobile users, SMPP servers, and custom USSD service providers.

## 🏗️ System Architecture

The system consists of three main components that work together to simulate a complete USSD ecosystem:

```
┌─────────────────┐    SMPP     ┌─────────────────┐    SMPP     ┌─────────────────┐
│                 │◄──────────►│                 │◄──────────►│                 │
│  User Simulator │             │  SMPP Server    │             │ Client Simulator│
│                 │             │   Simulator     │             │                 │
└─────────────────┘             └─────────────────┘             └─────────────────┘
    📱 Mobile User                   🌐 Gateway                      🏢 Service Provider
```

### 1. **USSD User Simulator** (`ussd_user_simulator`)
- **Role**: Simulates a mobile phone user dialing USSD codes
- **System ID**: `USSDMobileUser`
- **Features**:
  - Interactive phone interface with visual display
  - Support for standard USSD codes (*123#, *100#, *199#)
  - Custom USSD code testing
  - Performance statistics and monitoring
  - Connection status tracking

### 2. **SMPP Server Simulator** (`ussd_smpp_simulator`)
- **Role**: Acts as the telecom gateway/SMPP server
- **System ID**: `USSDGateway`
- **Features**:
  - Handles multiple client connections simultaneously
  - Routes USSD requests based on code patterns
  - Manages two distinct client groups:
    - **User Clients**: Mobile users initiating requests
    - **Forwarding Clients**: Service providers handling custom codes
  - Built-in responses for standard telecom services
  - Configurable response percentages for testing scenarios

### 3. **USSD Client Simulator** (`ussd_smpp_client_simulator`)
- **Role**: Simulates a custom USSD service provider
- **System ID**: `ForwardingClient`
- **Features**:
  - Handles custom USSD codes (e.g., *555*1#)
  - Provides interactive menu systems
  - Multi-level navigation support
  - Session management with state tracking
  - Custom service implementations

The client component provides:
- SMPP client implementation
- Interactive user simulation
- Automated test suite execution
- Configuration-driven operation

**Key Features:**
- Three operation modes: `user`, `test`, and `client`
- Interactive USSD session simulation
- Automated testing with configurable test cases
- Command-line argument support with configuration overrides

## 🚀 Quick Start

### Prerequisites
- Rust 1.70+ installed
- Terminal access

### Building the System

```bash
# Build all components
cd ussd_smpp_simulator && cargo build --release
cd ../ussd_smpp_client_simulator && cargo build --release
cd ../ussd_user_simulator && cargo build --release
```

### Running the System

1. **Start the SMPP Server** (Terminal 1):
```bash
cd ussd_smpp_simulator
./target/release/ussd_smpp_simulator
```

2. **Start the Client Simulator** (Terminal 2):
```bash
cd ussd_smpp_client_simulator
./target/release/ussd_smpp_client_simulator
```

3. **Start the User Simulator** (Terminal 3):
```bash
cd ussd_user_simulator
./target/release/ussd_user_simulator
```

## 📱 Testing the System

### Standard USSD Codes (Handled by Server)
- `*123#` - Main menu
- `*100#` - Balance check
- `*199#` - Data balance

### Custom USSD Codes (Forwarded to Client)
- `*555*1#` - Custom services menu
- `*555*2#` - Bank services
- `*555*3#` - Mobile services

### Usage Flow
1. Select option 4 "Custom USSD Code" in the user simulator
2. Enter a custom code like `*555*1#`
3. The system will:
   - User simulator sends SUBMIT_SM to server
   - Server forwards to client simulator
   - Client processes and returns menu
   - Server routes response back to user
   - User sees the custom menu

## ⚙️ Configuration

### Server Configuration (`ussd_smpp_simulator/config.toml`)
```toml
[client_simulator]
forwarding_clients = ["ForwardingClient", "JavaClient"]  # Service providers
user_clients = ["USSDMobileUser"]                        # Mobile users

[ussd]
service_codes = ["*199#","*123#","*100#"]  # Server-handled codes
```

### Client Configuration (`ussd_smpp_client_simulator/client_config.toml`)
```toml
[ussd_codes]
"*555*1#" = "main"  # Maps USSD codes to menu handlers
```

### User Configuration (`ussd_user_simulator/user_config.toml`)
```toml
[authentication]
system_id = "USSDMobileUser"  # Must match server's user_clients list
```

## Quick Start

### Prerequisites

- Rust (latest stable version)
- Cargo package manager

### Installation

1. Clone or download the project
2. Navigate to the project directory:
   ```bash
   cd /path/to/ussd-smpp-system
   ```

### Running the System

#### Method 1: Using the Integration Test Script

```bash
# Make the script executable
chmod +x test_integration.sh

# Run the complete integration test
./test_integration.sh
```

#### Method 2: Manual Setup

**Terminal 1 - Start the Server:**
```bash
cd ussd_smpp_simulator
cargo run
```

**Terminal 2 - Run the Client:**
```bash
cd ussd_client_simulator

# Interactive user simulation
cargo run -- user 1234567890

# Automated test suite
cargo run -- test

# Basic client mode
cargo run -- client 1234567890
```

## Integration Testing

### Automated Integration Test

The `test_integration.sh` script provides a complete end-to-end test:

```bash
#!/bin/bash

# USSD SMPP System Integration Test
# This script demonstrates the full USSD system working together

echo "=== USSD SMPP System Integration Test ==="
echo

# Kill any existing processes
pkill -f ussd_smpp_simulator || true
pkill -f ussd_client_simulator || true
sleep 1

echo "Starting USSD SMPP Server..."
cd ussd_smpp_simulator
cargo run &
SERVER_PID=$!
cd ..

echo "Waiting for server to start..."
sleep 3

echo "Running USSD Client Test Suite..."
cd ussd_client_simulator
timeout 30 cargo run -- test
TEST_RESULT=$?
cd ..

echo
echo "Stopping server..."
kill $SERVER_PID 2>/dev/null || true
wait $SERVER_PID 2>/dev/null || true

if [ $TEST_RESULT -eq 0 ]; then
    echo "✓ Integration test PASSED"
else
    echo "✗ Integration test FAILED"
fi

echo
echo "=== Test Complete ==="
```

### Manual Testing Steps

1. **Start the Server:**
   ```bash
   cd ussd_smpp_simulator
   cargo run
   ```

2. **Test Basic Connectivity:**
   ```bash
   cd ussd_client_simulator
   cargo run -- client 1234567890
   ```

3. **Run Interactive Session:**
   ```bash
   cargo run -- user 1234567890
   ```
   - Enter `*123#` to start
   - Navigate through the menu options
   - Test various scenarios

4. **Execute Test Suite:**
   ```bash
   cargo run -- test
   ```

## USSD Flow Examples

### Basic USSD Session

```
User → Server: *123#
Server → User: Welcome to MyTelecom USSD Service
               1. Balance Inquiry
               2. Data Packages
               3. Customer Service
               0. Exit

User → Server: 1
Server → User: Your current balance is $25.50
               Your data balance is 2.5GB
               Press 0 to return to main menu

User → Server: 0
Server → User: [Returns to main menu]

User → Server: 0
Server → User: Thank you for using MyTelecom USSD Service. Goodbye!
```

### Data Package Selection

```
User → Server: *123#
Server → User: [Main menu]

User → Server: 2
Server → User: Available Data Packages:
               1. 1GB Package - $10.00
               2. 5GB Package - $40.00
               3. 10GB Package - $70.00
               0. Back to main menu

User → Server: 1
Server → User: You have selected: 1GB Package
               Price: $10.00
               Type YES to confirm or NO to cancel

User → Server: YES
Server → User: Package activated successfully!
               Thank you for your purchase.
```

## Development

### Building

```bash
# Build both components
cargo build --release

# Build individual components
cd ussd_smpp_simulator && cargo build --release
cd ussd_client_simulator && cargo build --release
```

### Testing

```bash
# Run unit tests
cargo test

# Run integration tests
./test_integration.sh

# Run with debug logging
# Edit config files to set debug = true
```

### Project Structure

```
ussd-smpp-system/
├── ussd_smpp_simulator/
│   ├── src/
│   │   └── main.rs
│   ├── Cargo.toml
│   ├── config.toml
│   └── README.md
├── ussd_client_simulator/
│   ├── src/
│   │   └── main.rs
│   ├── Cargo.toml
│   ├── client_config.toml
│   └── README.md
├── test_integration.sh
└── README.md
```

## Protocol Details

### SMPP Implementation

- **Version**: SMPP v3.4
- **Transport**: TCP/IP
- **Default Port**: 9090
- **Supported Operations**:
  - `BIND_TRANSCEIVER` / `BIND_TRANSCEIVER_RESP`
  - `SUBMIT_SM` / `SUBMIT_SM_RESP`
  - `DELIVER_SM` / `DELIVER_SM_RESP`
  - `ENQUIRE_LINK` / `ENQUIRE_LINK_RESP`
  - `UNBIND` / `UNBIND_RESP`

### USSD Protocol

- **Service Type**: USSD
- **ESM Class**: 0x40 (USSD indication)
- **Data Coding**: GSM 7-bit (default)
- **Message Flow**: Request → Response → [Continue session or terminate]

## 🚨 Troubleshooting

### Common Issues

1. **Address already in use**
   ```bash
   pkill -f ussd_smpp_simulator
   ```

2. **Client connection refused**
   - Ensure server is running first
   - Check port configuration (default: 2775)

3. **No response from custom codes**
   - Verify client simulator is running
   - Check `forwarding_clients` configuration
   - Ensure USSD code is mapped in client config

4. **Permission denied**
   ```bash
   chmod +x target/release/ussd_*
   ```

### Debug Mode
Enable debug logging in configuration files:
```toml
[logging]
debug = true
```

This provides detailed information about:
- SMPP PDU exchanges
- USSD session states
- Configuration loading
- Error details

## License

## 📊 Performance Testing

The system includes built-in performance testing capabilities:

- Response time monitoring
- Success/failure rate tracking  
- Connection stability testing
- Load testing with configurable response percentages

## 🛠️ Development

### Project Structure
```
├── ussd_smpp_simulator/          # SMPP Server
│   ├── src/main.rs               # Server logic
│   ├── config.toml               # Server configuration
│   └── Cargo.toml
├── ussd_smpp_client_simulator/   # Client Simulator
│   ├── src/
│   │   ├── main.rs              # Client logic
│   │   ├── smpp.rs              # SMPP handling
│   │   └── ussd.rs              # USSD menu system
│   ├── client_config.toml       # Client configuration
│   └── Cargo.toml
├── ussd_user_simulator/          # User Simulator
│   ├── src/main.rs              # User interface
│   ├── user_config.toml         # User configuration
│   └── Cargo.toml
└── README.md                     # This file
```

### Adding New Services
1. Add new USSD codes to client configuration
2. Implement menu handlers in `ussd.rs`
3. Update routing logic if needed
4. Test with user simulator

### Testing Scripts
- `test_complete_forwarding.py` - End-to-end testing
- `test_integration.sh` - Integration testing
- `test_user_simulator.sh` - User simulator testing

## 🤝 Contributing

Feel free to submit issues and enhancement requests!

---

**Note**: This is a simulation system for development and testing purposes. It demonstrates USSD/SMPP protocol interactions and should not be used in production environments without proper security and reliability enhancements.
