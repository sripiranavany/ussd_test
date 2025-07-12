# USSD User Simulator

An advanced USSD (Unstructured Supplementary Service Data) user simulator that connects to SMPP servers for testing and development purposes.

## Features

### Core Features
- **Real SMPP Connectivity**: Connects to SMPP servers using the SMPP protocol
- **Interactive Mobile Phone UI**: Realistic mobile phone interface simulation
- **Configurable Parameters**: Comprehensive configuration via TOML files
- **Performance Monitoring**: Real-time statistics and performance metrics
- **Connection Management**: Automatic reconnection with configurable retry logic
- **Session Management**: Timeout handling and session state tracking

### Advanced Features
- **Test Scenarios**: Automated testing with predefined scenarios
- **Load Testing**: Performance testing with concurrent sessions
- **Connection Health Monitoring**: Real-time connection status and uptime tracking
- **Error Handling**: Comprehensive error handling with detailed logging
- **Command Line Interface**: Full CLI support with argument parsing
- **Debug Mode**: Extensive debug logging for troubleshooting

## Installation

### Prerequisites
- Rust 1.70+ (2021 edition)
- A running USSD SMPP server (e.g., `ussd_smpp_simulator`)

### Build
```bash
cargo build --release
```

## Configuration

The simulator uses a TOML configuration file (`user_config.toml`) with the following sections:

### Server Configuration
```toml
[server]
host = "127.0.0.1"                    # SMPP server host
port = 9090                           # SMPP server port
connection_timeout_ms = 5000          # Connection timeout
reconnect_attempts = 3                # Number of reconnection attempts
keepalive_interval_ms = 30000         # Keepalive interval
```

### Authentication
```toml
[authentication]
system_id = "USSDMobileUser"          # SMPP system ID
password = "mobile123"                # SMPP password
system_type = "USSD"                  # System type
```

### Phone Configuration
```toml
[phone]
default_msisdn = "1234567890"         # Phone number
operator_name = "MyTelecom"           # Operator name
balance = 25.5                        # Account balance
data_balance = 2.5                    # Data balance (GB)
country_code = "1"                    # Country code
network_code = "001"                  # Network code
```

### UI Settings
```toml
[ui]
animation_delay_ms = 800              # Animation delay
auto_clear_screen = true              # Auto-clear screen
show_debug_info = false               # Show debug info
show_performance_stats = true         # Show performance statistics
session_timeout_ms = 30000            # Session timeout
max_input_length = 160                # Maximum input length
```

### Logging
```toml
[logging]
debug = false                         # Enable debug logging
log_file = "ussd_simulator.log"       # Log file path
log_level = "info"                    # Log level
enable_file_logging = true            # Enable file logging
```

### Testing
```toml
[testing]
auto_test_on_startup = false          # Auto-run tests on startup
test_scenarios_file = "test_scenarios.toml"  # Test scenarios file
performance_test_enabled = false      # Enable performance testing
concurrent_sessions = 1               # Number of concurrent sessions
```

### Advanced Settings
```toml
[advanced]
smpp_version = "3.4"                  # SMPP protocol version
enquire_link_interval_ms = 60000      # Enquire link interval
pdu_timeout_ms = 10000                # PDU timeout
max_concurrent_requests = 5           # Maximum concurrent requests
```

## Usage

### Basic Usage
```bash
# Run with default configuration
./ussd_user_simulator

# Run with custom configuration
./ussd_user_simulator --config custom_config.toml

# Run with debug mode
./ussd_user_simulator --debug

# Override specific settings
./ussd_user_simulator --host 192.168.1.100 --port 9999 --msisdn 9876543210
```

### Command Line Options
```
Options:
  -c, --config <CONFIG>    Path to configuration file (default: user_config.toml)
  -m, --msisdn <MSISDN>    Override phone number from config
  -h, --host <HOST>        Override server host from config
  -p, --port <PORT>        Override server port from config
  --create-config          Create a default config file and exit
  --debug                  Enable debug mode
  --help                   Show help message
```

### Interactive Menu

The simulator provides an interactive menu with the following options:

1. **Main Menu (*123#)** - Access the main USSD menu
2. **Balance Check (*100#)** - Check account balance
3. **Data Balance (*199#)** - Check data balance
4. **Custom USSD Code** - Enter custom USSD codes
5. **Performance Stats** - View performance statistics
6. **Connection Test** - Test SMPP connection
7. **Run Test Scenarios** - Execute predefined test scenarios
8. **Exit** - Exit the simulator

### Performance Statistics

The simulator tracks and displays:
- Total requests sent
- Successful/failed request counts
- Success rate percentage
- Average response time
- Fastest and slowest response times
- Connection uptime
- Server information

### Test Scenarios

The simulator includes automated test scenarios:
- **Main Menu Navigation** - Test menu navigation flows
- **Balance Check Flow** - Test balance inquiry functionality
- **Data Balance Flow** - Test data balance checks
- **Service Menu Navigation** - Test service menus
- **Error Handling Test** - Test error conditions
- **Performance Test** - Test rapid requests

## Test Scenarios Configuration

Create a `test_scenarios.toml` file to define custom test scenarios:

```toml
[[scenarios]]
name = "Balance Check Flow"
description = "Test balance inquiry functionality"
timeout_ms = 15000
expected_success_rate = 98.0

[[scenarios.steps]]
ussd_code = "*100#"
description = "Direct balance check"
expected_keywords = ["balance", "$", "current"]
timeout_ms = 5000
```

## Integration with SMPP Server

### Start SMPP Server
```bash
# In the ussd_smpp_simulator directory
cd ../ussd_smpp_simulator
cargo run --bin ussd_smpp_simulator
```

### Test Connection
```bash
# In the ussd_user_simulator directory
cargo run --bin ussd_user_simulator -- --debug
```

## Performance Monitoring

### Real-time Statistics
- Connection uptime
- Request/response metrics
- Success/failure rates
- Response time analysis

### Performance Testing
- Concurrent session testing
- Load testing capabilities
- Stress testing scenarios
- Performance reporting

## Troubleshooting

### Common Issues

1. **Connection Refused**
   - Ensure SMPP server is running
   - Check host and port configuration
   - Verify network connectivity

2. **Authentication Failed**
   - Check system_id and password
   - Verify server configuration
   - Review authentication logs

3. **Timeout Errors**
   - Increase timeout values in config
   - Check network latency
   - Verify server responsiveness

4. **Performance Issues**
   - Reduce concurrent sessions
   - Increase timeouts
   - Monitor system resources

### Debug Mode
Enable debug mode for detailed logging:
```bash
./ussd_user_simulator --debug
```

### Log Files
Check log files for detailed error information:
```bash
tail -f ussd_simulator.log
```

## Development

### Project Structure
```
ussd_user_simulator/
├── src/
│   └── main.rs              # Main application code
├── Cargo.toml               # Dependencies and metadata
├── user_config.toml         # Default configuration
├── test_scenarios.toml      # Test scenarios
└── README.md               # This file
```

### Key Components
- **UssdMobileUI** - User interface and interaction logic
- **UssdSmppClient** - SMPP protocol implementation
- **PerformanceStats** - Performance monitoring
- **Configuration** - Settings management

### Contributing
1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## Testing

### Unit Tests
```bash
cargo test
```

### Integration Tests
```bash
# Start SMPP server first
cd ../ussd_smpp_simulator && cargo run &

# Run user simulator tests
cd ../ussd_user_simulator
cargo run -- --debug
```

### Performance Tests
```bash
# Enable performance testing in config
cargo run -- --config performance_config.toml
```

## Security Considerations

- Configure appropriate authentication credentials
- Use secure connections in production
- Implement rate limiting if needed
- Monitor for unusual activity
- Keep logs secure and rotate regularly

## License

This project is licensed under the MIT License.

## Support

For issues and questions:
1. Check the troubleshooting section
2. Review log files
3. Enable debug mode for detailed information
4. Report issues with detailed logs and configuration
