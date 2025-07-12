# Quick Start Guide

## 1. Build the Application
```bash
cargo build --release
```

## 2. Create Configuration
```bash
# Create default configuration
./target/release/ussd_smpp_simulator --create-config

# This creates config.toml with default settings
```

## 3. Start the Server
```bash
# Using default config
./target/release/ussd_smpp_simulator

# Using custom config
./target/release/ussd_smpp_simulator --config prod.toml

# With overrides
./target/release/ussd_smpp_simulator --host 0.0.0.0 --port 8080
```

## 4. Test Different Configurations

### Development Mode
```bash
./target/release/ussd_smpp_simulator --config dev.toml
```
- Service code: *999#
- Debug logging enabled
- Local binding (127.0.0.1)

### Production Mode
```bash
./target/release/ussd_smpp_simulator --config prod.toml
```
- Service code: *100#
- Binds to all interfaces (0.0.0.0)
- Higher connection limits

## 5. Configuration Examples

### Basic Configuration (config.toml)
- Host: 127.0.0.1
- Port: 2775
- Service Code: *123#
- Debug: Off

### Development (dev.toml)
- Debug mode enabled
- Test service code: *999#
- Lower timeouts for faster testing

### Production (prod.toml)
- External binding: 0.0.0.0
- Higher connection limits
- Production service code: *100#

## 6. Command Line Options

```bash
# Get help
./target/release/ussd_smpp_simulator --help

# Create config
./target/release/ussd_smpp_simulator --create-config

# Override settings
./target/release/ussd_smpp_simulator -c config.toml -h 0.0.0.0 -p 2775
```

## 7. Customization

Edit any .toml file to customize:
- USSD menus and messages
- Data packages and pricing
- Service codes
- Connection settings
- Debug options

## 8. Testing

Use an SMPP client to connect to the configured host:port and test USSD functionality.

Default USSD flow:
1. Send SUBMIT_SM with service code (*123#, *999#, or *100#)
2. Receive menu response
3. Send user selections
4. Navigate through menus
