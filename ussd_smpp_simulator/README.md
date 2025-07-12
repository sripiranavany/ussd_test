# USSD SMPP Simulator

A USSD (Unstructured Supplementary Service Data) SMPP (Short Message Peer-to-Peer) simulator written in Rust.

## Features

- SMPP protocol implementation
- USSD session management
- Interactive menu system
- **Configuration file support (TOML format)**
- Configurable host and port
- Multi-threaded connection handling
- Debug logging
- Command-line configuration overrides

## Building

```bash
cargo build --release
```

## Configuration

The simulator uses a TOML configuration file for easy customization. The default configuration file is `config.toml`.

### Creating a Configuration File

```bash
# Create a default configuration file
./target/release/ussd_smpp_simulator --create-config
```

This creates a `config.toml` file with default settings that you can customize.

### Configuration Structure

```toml
[server]
host = "127.0.0.1"      # Host to bind to
port = 2775             # Port to bind to

[smpp]
system_id = "USSDGateway"    # SMPP System ID
max_connections = 100        # Maximum concurrent connections
connection_timeout = 300     # Connection timeout in seconds

[ussd]
service_code = "*123#"       # USSD service code
session_timeout = 180        # Session timeout in seconds

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

[[ussd.data_packages.packages]]
name = "5GB Package"
price = 40.0
data = "5GB"

[logging]
debug = false           # Enable debug logging
log_file = ""          # Log file path (empty for console only)
```

## Usage

### Default Configuration
```bash
./target/release/ussd_smpp_simulator
```
This uses the default `config.toml` file.

### Custom Configuration File
```bash
./target/release/ussd_smpp_simulator --config /path/to/custom.toml
```

### Command-Line Overrides
```bash
# Override host and port from config
./target/release/ussd_smpp_simulator --host 0.0.0.0 --port 8080

# Use custom config with overrides
./target/release/ussd_smpp_simulator -c myconfig.toml --host 192.168.1.100
```

### Help
```bash
./target/release/ussd_smpp_simulator --help
```

## Configuration Options

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| --config | -c | Path to configuration file | config.toml |
| --host | -h | Override host from config | - |
| --port | -p | Override port from config | - |
| --create-config | | Create default config file | - |
| --help | | Show help message | - |

## USSD Menu Structure

The menu structure is fully configurable through the configuration file. The default menu provides:

1. **Main Menu** (accessed via configurable service code, default `*123#`)
   - Balance Inquiry
   - Data Packages
   - Customer Service
   - Exit

2. **Balance Inquiry**
   - Shows configurable balance message
   - Option to return to main menu

3. **Data Packages**
   - Dynamically generated from configuration
   - Configurable packages with name, price, and data amount
   - Purchase confirmation via "YES"

4. **Customer Service**
   - Contact information
   - Option to return to main menu

## SMPP Protocol Support

The simulator supports the following SMPP operations:

- BIND_RECEIVER
- BIND_TRANSMITTER
- BIND_TRANSCEIVER
- SUBMIT_SM
- DELIVER_SM
- UNBIND
- ENQUIRE_LINK

## Examples

### Basic Usage
```bash
./target/release/ussd_smpp_simulator
```

### Create and Edit Configuration
```bash
# Create default config
./target/release/ussd_smpp_simulator --create-config

# Edit config.toml to customize settings
# Then run with custom settings
./target/release/ussd_smpp_simulator
```

### External Access
```bash
./target/release/ussd_smpp_simulator --host 0.0.0.0 --port 2775
```

### Debug Mode
```bash
# Enable debug in config.toml by setting debug = true
./target/release/ussd_smpp_simulator
```

### Multiple Configurations
```bash
# Production config
./target/release/ussd_smpp_simulator --config prod.toml

# Development config
./target/release/ussd_smpp_simulator --config dev.toml --host 127.0.0.1
```

## Development

The simulator is built with Rust and uses minimal external dependencies:

- **serde**: For configuration serialization/deserialization
- **toml**: For TOML configuration file parsing

### Project Structure
```
src/
├── main.rs          # Main application logic
config.toml          # Configuration file
Cargo.toml           # Project configuration
```

### Customization

You can easily customize the simulator by:

1. **Modifying the configuration file** to change messages, menu options, data packages, etc.
2. **Setting different service codes** for different USSD services
3. **Configuring multiple instances** with different config files
4. **Adding custom data packages** with different pricing
5. **Customizing all user-facing messages**

## License

This project is for educational and testing purposes.
