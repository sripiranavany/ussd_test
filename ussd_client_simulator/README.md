# USSD Client Simulator

This is a USSD client simulator that connects to the USSD SMPP Simulator to test USSD functionality.

## Features

- Interactive user simulation
- Automated test suite
- Basic client mode
- Configuration file support
- Command-line argument overrides

## Configuration

The client uses a configuration file (`client_config.toml` by default) to manage settings:

### Configuration Sections

- **server**: Server connection settings (host, port)
- **authentication**: Credentials for binding to the SMPP server
- **defaults**: Default values for MSISDN, USSD codes, and delays
- **test_cases**: Test scenarios for automated testing
- **logging**: Debug and logging settings

### Sample Configuration

```toml
[server]
host = "127.0.0.1"
port = 2775

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

## Usage

### Basic Usage

```bash
# Interactive user simulation
./ussd_client_simulator user 1234567890

# Run automated test suite
./ussd_client_simulator test

# Basic client mode
./ussd_client_simulator client 1234567890
```

### With Configuration Options

```bash
# Use custom configuration file
./ussd_client_simulator -c /path/to/config.toml user 1234567890

# Override server settings
./ussd_client_simulator --host 192.168.1.100 --port 2776 test

# Create default configuration file
./ussd_client_simulator --create-config

# Enable debug mode (in config file)
# Set logging.debug = true in client_config.toml
```

### Command Line Options

- `-c, --config <CONFIG>`: Path to configuration file
- `-h, --host <HOST>`: Override server host
- `-p, --port <PORT>`: Override server port
- `--create-config`: Create default configuration file
- `--help`: Show help message

### Modes

1. **user**: Interactive user simulation
   - Prompts for user input
   - Simulates real USSD session
   - Can navigate through menus

2. **test**: Automated test suite
   - Runs predefined test cases
   - Tests various USSD scenarios
   - Reports pass/fail results

3. **client**: Basic client mode
   - Simple USSD request/response
   - Good for basic testing

## Connection to USSD SMPP Simulator

This client simulator is designed to connect to the USSD SMPP Simulator server. Both use the same default configuration:

- **Server**: `127.0.0.1:2775`
- **Protocol**: SMPP v3.4
- **Service**: USSD over SMPP

### Starting Both Components

1. Start the USSD SMPP Server:
   ```bash
   cd ../ussd_smpp_simulator
   cargo run
   ```

2. Start the USSD Client Simulator:
   ```bash
   cd ../ussd_client_simulator
   cargo run user 1234567890
   ```

## Building

```bash
cargo build --release
```

## Examples

### Interactive Session
```bash
./ussd_client_simulator user 1234567890
```

This will:
1. Connect to the USSD SMPP server
2. Send the initial USSD code (*123#)
3. Display the response
4. Prompt for user input
5. Continue the session until terminated

### Test Suite
```bash
./ussd_client_simulator test
```

This will run all configured test cases and report results.

### Custom Configuration
```bash
./ussd_client_simulator -c my_config.toml --host 192.168.1.100 user 9876543210
```

This uses a custom config file and overrides the server host.
