<!-- Use this file to provide workspace-specific custom instructions to Copilot. For more details, visit https://code.visualstudio.com/docs/copilot/copilot-customization#_use-a-githubcopilotinstructionsmd-file -->

# USSD SMPP Client Simulator

This is a Rust-based SMPP client simulator that acts as a forwarding client for custom USSD codes. The client connects to a USSD SMPP server and handles custom USSD menu interactions.

## Project Structure

- This is a Rust binary project using Cargo
- Uses TOML for configuration management
- Implements SMPP 3.4 protocol for communication
- Supports configurable custom USSD menu systems
- Handles both synchronous and asynchronous SMPP operations

## Key Features

- SMPP client that can bind as a forwarding client
- Configurable custom USSD menu definitions
- Support for nested menu structures
- Dynamic response generation based on user input
- Error handling and graceful degradation
- Debug logging and monitoring capabilities

## Development Guidelines

- Use standard Rust error handling patterns with Result types
- Implement proper SMPP PDU parsing and generation
- Use serde for configuration serialization/deserialization
- Follow async/await patterns for network operations
- Include comprehensive logging for debugging
- Maintain clean separation between SMPP protocol and business logic

## Configuration

The client uses TOML configuration files to define:
- Server connection parameters
- SMPP binding credentials
- Custom USSD menu structures
- Response templates and behaviors
- Logging and debugging options
