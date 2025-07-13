# USSD SMPP Client Simulator

A configurable Rust-based SMPP client simulator that acts as a forwarding client for custom USSD codes. This client connects to a USSD SMPP server and provides sophisticated menu-driven responses to USSD requests.

## Features

- **🔗 SMPP 3.4 Protocol Support**: Full implementation of SMPP client functionality
- **📱 Configurable USSD Menus**: Complex nested menu structures with customizable options
- **🎯 Dynamic Response System**: Template-based responses with rich formatting
- **🔄 Auto-reconnection**: Automatic reconnection on connection failures
- **📊 Session Management**: Timeout handling and session persistence
- **🔧 Flexible Configuration**: TOML-based configuration with hot-reloading
- **📝 Comprehensive Logging**: Debug and info logging for monitoring

## Quick Start

### Prerequisites

- Rust 1.70 or higher
- Access to a USSD SMPP server (like the companion `ussd_smpp_simulator`)

### Building

```bash
cargo build --release
```

### Running

```bash
# Run with default configuration
cargo run

# Run with custom configuration
cargo run -- --config my_config.toml

# Run with debug logging
cargo run -- --debug
```

## Configuration

The client is configured via a TOML file (`client_config.toml` by default). Here's the structure:

### Client Settings

```toml
[client]
host = "127.0.0.1"              # SMPP server host
port = 2775                     # SMPP server port
system_id = "ForwardingClient"  # Client system ID
password = "forward123"         # Authentication password
bind_type = "transceiver"       # Bind type
auto_reconnect = true           # Auto-reconnect on failures
heartbeat_interval = 30         # Heartbeat interval in seconds
```

### Menu Configuration

Define nested menu structures with customizable options:

```toml
[menus.main]
title = "🏠 Main Menu"
options = [
    { key = "1", text = "💰 Banking", action = "submenu", target = "banking" },
    { key = "2", text = "📱 Mobile", action = "submenu", target = "mobile" },
    { key = "0", text = "❌ Exit", action = "exit", target = "" }
]

[menus.banking]
title = "💰 Banking Services"
options = [
    { key = "1", text = "💳 Balance", action = "response", target = "balance" },
    { key = "2", text = "💸 Transfer", action = "submenu", target = "transfer" },
    { key = "0", text = "🔙 Back", action = "submenu", target = "main" }
]
```

### Response Templates

Define rich response templates:

```toml
[responses]
balance = """
💰 Your Account Balance:

💳 Savings: $2,450.75
💼 Current: $1,230.50
💎 Fixed Deposit: $10,000.00

📅 Last Updated: Today 2:30 PM
"""
```

### Session Management

```toml
[session]
timeout_seconds = 300           # Session timeout
max_menu_depth = 10            # Maximum menu nesting
enable_back_navigation = true   # Enable "00" back navigation
remember_last_menu = true      # Remember user's last menu
```

## Menu Actions

The client supports three types of menu actions:

1. **`submenu`**: Navigate to another menu
2. **`response`**: Show a predefined response
3. **`exit`**: End the session

## Navigation

- **Number keys (1-9)**: Select menu options
- **00**: Go back to previous menu (if enabled)
- **0**: Usually exit or main menu (depending on configuration)

## Logging

Set logging level in configuration:

```toml
[logging]
level = "info"    # "debug", "info", "warn", "error"
debug = false     # Enable debug mode
```

Or override with command line:

```bash
cargo run -- --debug
```

## Architecture

The client consists of several modules:

- **`main.rs`**: Application entry point and core logic
- **`smpp.rs`**: SMPP protocol implementation
- **`ussd.rs`**: USSD menu management and session handling
- **`config.rs`**: Configuration management

## Integration

This client is designed to work with the USSD SMPP server simulator. When the server receives a custom USSD code (not in its service codes), it forwards the request to this client via SMPP protocol.

### Flow:
1. User dials custom USSD code (e.g., `*555#`)
2. Server forwards request to this client via SUBMIT_SM
3. Client processes through configured menus
4. Client sends response back via DELIVER_SM
5. Server forwards response to user

## Development

### Running Tests

```bash
cargo test
```

### Debug Mode

For detailed logging:

```bash
RUST_LOG=debug cargo run -- --debug
```

### Adding New Menu Types

Extend the `MenuOption` action types in `config.rs` and implement handling in `ussd.rs`.

## Examples

### Simple Banking Menu

```toml
[menus.banking]
title = "🏦 Banking Services"
options = [
    { key = "1", text = "💰 Check Balance", action = "response", target = "balance" },
    { key = "2", text = "💸 Transfer Money", action = "submenu", target = "transfer" },
    { key = "3", text = "📊 Statement", action = "response", target = "statement" },
    { key = "0", text = "🔙 Main Menu", action = "submenu", target = "main" }
]
```

### Dynamic Response

```toml
[responses]
balance = """
💰 Account Balance

💳 Checking: $1,234.56
💰 Savings: $5,678.90
💎 Investment: $10,000.00

📅 As of: {current_time}
💳 Available: $6,913.46
"""
```

## Troubleshooting

### Connection Issues

1. Verify SMPP server is running
2. Check host/port configuration
3. Ensure system_id is in server's forwarding_clients list
4. Check firewall settings

### Menu Navigation Issues

1. Verify menu structure in configuration
2. Check for circular references
3. Ensure all target menus exist
4. Verify response templates are defined

### Performance Issues

1. Adjust session timeout settings
2. Implement session cleanup
3. Monitor memory usage
4. Check network latency

## Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Submit a pull request

## License

This project is licensed under the MIT License.
