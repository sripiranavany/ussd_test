# USSD SMPP Simulator System

A complete USSD (Unstructured Supplementary Service Data) simulation system built in Rust, consisting of both server and client components that communicate via the SMPP (Short Message Peer-to-Peer) protocol.

## Overview

This system simulates a telecommunications USSD service, allowing you to test USSD interactions without requiring actual mobile network infrastructure. It includes:

- **USSD SMPP Server**: Handles USSD requests and provides menu-driven responses
- **USSD Client Simulator**: Simulates mobile devices making USSD requests
- **Configuration Management**: Flexible TOML-based configuration for both components
- **Testing Framework**: Automated test suites and manual testing capabilities

## System Architecture

```
┌─────────────────────┐    SMPP v3.4     ┌─────────────────────┐
│  USSD Client        │◄────────────────►│  USSD SMPP Server   │
│  Simulator          │   TCP/IP :9090   │                     │
├─────────────────────┤                  ├─────────────────────┤
│ • User Simulation   │                  │ • SMPP Protocol     │
│ • Test Suite        │                  │ • USSD Menu System  │
│ • Basic Client      │                  │ • Session Management│
│ • Configuration     │                  │ • Configuration     │
└─────────────────────┘                  └─────────────────────┘
```

## Components

### 1. USSD SMPP Server (`ussd_smpp_simulator/`)

The server component provides:
- SMPP v3.4 protocol implementation
- Multi-threaded connection handling
- USSD menu system with configurable responses
- Session management and state tracking
- Comprehensive logging and debugging

**Key Features:**
- Configurable USSD menus (Balance inquiry, Data packages, Customer service)
- Dynamic response generation based on user input
- Support for multiple concurrent sessions
- Flexible server configuration (host, port, credentials)

### 2. USSD Client Simulator (`ussd_client_simulator/`)

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

## Configuration

### Server Configuration (`ussd_smpp_simulator/config.toml`)

```toml
[server]
host = "0.0.0.0"
port = 9090

[smpp]
system_id = "USSDGateway"
max_connections = 100
connection_timeout = 300

[ussd]
service_code = "*123#"
session_timeout = 180

[ussd.menu]
welcome_message = "Welcome to MyTelecom USSD Service"
main_menu = [
    "1. Balance Inquiry",
    "2. Data Packages", 
    "3. Customer Service",
    "0. Exit"
]

[ussd.responses]
balance_message = "Your current balance is $25.50\nYour data balance is 2.5GB"
invalid_code = "Invalid USSD code. Please try again."
invalid_option = "Invalid option. Please try again."
goodbye_message = "Thank you for using MyTelecom USSD Service. Goodbye!"

[[ussd.data_packages.packages]]
name = "1GB Package"
price = 10.0
data = "1GB"

[logging]
debug = true
log_file = ""
```

### Client Configuration (`ussd_client_simulator/client_config.toml`)

```toml
[server]
host = "127.0.0.1"
port = 9090

[authentication]
system_id = "USSDClient"
password = "password123"
test_system_id = "USSDTestClient"
test_password = "testpass123"

[defaults]
default_msisdn = "1234567890"
initial_ussd_code = "*123#"
request_delay_ms = 500

[logging]
debug = false
log_file = ""

[[test_cases.test_cases]]
msisdn = "1234567890"
ussd_code = "*123#"
description = "Test main menu access"
```

## Usage Examples

### Server Usage

```bash
# Start with default configuration
cargo run

# Start with custom configuration
cargo run -- -c /path/to/custom_config.toml

# Override host and port
cargo run -- --host 192.168.1.100 --port 2775

# Create default configuration file
cargo run -- --create-config
```

### Client Usage

```bash
# Interactive user simulation
cargo run -- user 1234567890

# Automated test suite
cargo run -- test

# Basic client mode
cargo run -- client 1234567890

# With custom configuration
cargo run -- -c custom_config.toml user 9876543210

# Override server settings
cargo run -- --host 192.168.1.100 --port 2775 test

# Create default configuration
cargo run -- --create-config
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

## Troubleshooting

### Common Issues

1. **Connection Refused**
   - Ensure server is running
   - Check host/port configuration
   - Verify firewall settings

2. **Bind Failures**
   - Check system_id and password in configuration
   - Verify server authentication settings

3. **Configuration Errors**
   - Use `--create-config` to generate default configurations
   - Validate TOML syntax

4. **Port Already in Use**
   - Kill existing processes: `pkill -f ussd_smpp_simulator`
   - Change port in configuration files

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

This project is provided as-is for educational and testing purposes.

## Contributing

Feel free to submit issues and enhancement requests!
