# USSD Code Configuration Guide

## Overview

The USSD SMPP Client Simulator supports configurable custom USSD codes that can be mapped to specific menus and responses. This document explains how to configure and use custom USSD codes.

## Configuration Structure

### 1. USSD Codes Section

The `[ussd_codes]` section in `client_config.toml` defines how USSD codes are handled:

```toml
[ussd_codes]
# Default menu for unrecognized USSD codes
default_menu = "main"

# Define specific USSD codes and their target menus
codes = [
    { code = "*999#", menu = "main", description = "Main Services Menu" },
    { code = "*100#", menu = "banking", description = "Banking Services" },
    { code = "*200#", menu = "mobile", description = "Mobile Services" },
    { code = "*300#", menu = "utilities", description = "Utilities" },
    { code = "*400#", menu = "support", description = "Help & Support" },
    { code = "*123#", menu = "main", description = "Alternative Main Menu" },
]

# Configure which USSD codes this client should handle
handle_codes = ["*999#", "*100#", "*200#", "*300#", "*400#", "*123#"]

# What to do with unrecognized USSD codes
unrecognized_action = "forward"  # Options: "forward", "reject", "default_menu"
unrecognized_message = "🚫 USSD code not recognized. Please try *999# for main menu."
```

### 2. Configuration Fields

#### `default_menu`
- The default menu to show when no specific mapping is found
- Should match a menu name defined in the `[menus]` section

#### `codes`
- Array of USSD code mappings
- Each mapping contains:
  - `code`: The USSD code (e.g., "*999#")
  - `menu`: The target menu name
  - `description`: Human-readable description

#### `handle_codes`
- List of USSD codes that this client will handle
- If empty, handles all codes defined in `codes`
- If specified, only handles codes in this list
- Codes not in this list will be handled according to `unrecognized_action`

#### `unrecognized_action`
- `"forward"`: Forward to the network/gateway (default behavior)
- `"reject"`: Reject the code with an error message
- `"default_menu"`: Redirect to the default menu

#### `unrecognized_message`
- Message to show when handling unrecognized codes
- Used with all `unrecognized_action` options

## How USSD Codes Work

### 1. Processing Flow

1. **USSD Code Received**: When a USSD code is received (format `*xxx#`)
2. **Handle Check**: Check if the code is in the `handle_codes` list (if specified)
3. **Mapping Lookup**: Look for a specific mapping in the `codes` array
4. **Action Taken**:
   - If found: Navigate to the specified menu
   - If not found: Use `default_menu`
   - If not handled: Apply `unrecognized_action`

### 2. Menu Navigation

- USSD codes always reset the session to the beginning
- Users can navigate through menus using numeric keys
- Special navigation:
  - `0`: Usually "Back" or "Exit"
  - `00`: Universal "Back" (if enabled)

### 3. Session Management

- Each user (MSISDN) has a separate session
- Sessions timeout after configured period
- Session state includes current menu and navigation history

## Example Scenarios

### Scenario 1: Direct Service Access

```toml
{ code = "*100#", menu = "banking", description = "Direct Banking Access" }
```

When user dials `*100#`, they go directly to the banking menu, skipping the main menu.

### Scenario 2: Alternative Entry Points

```toml
{ code = "*999#", menu = "main", description = "Main Services Menu" },
{ code = "*123#", menu = "main", description = "Alternative Main Menu" },
```

Both `*999#` and `*123#` lead to the same main menu.

### Scenario 3: Restricted Handling

```toml
handle_codes = ["*999#", "*100#"]
unrecognized_action = "reject"
```

Only `*999#` and `*100#` are handled. Other codes are rejected with an error message.

## Testing USSD Codes

### 1. Using the Test Script

```bash
python3 test_rust_client.py
```

This will test various USSD codes and menu navigation.

### 2. Manual Testing

1. Start the server: `cd ussd_smpp_simulator && cargo run`
2. Start the client: `cd ussd_smpp_client_simulator && cargo run`
3. Use the Python test scripts to send USSD requests

### 3. Supported Test Codes

Based on the default configuration:

- `*999#` → Main Services Menu
- `*100#` → Banking Services
- `*200#` → Mobile Services
- `*300#` → Utilities
- `*400#` → Help & Support
- `*123#` → Alternative Main Menu

## Advanced Configuration

### 1. Dynamic Menu Mapping

You can create specialized entry points:

```toml
{ code = "*BAL#", menu = "balance_check", description = "Quick Balance Check" },
{ code = "*PAY#", menu = "bill_payment", description = "Quick Bill Payment" },
```

### 2. Service-Specific Codes

Group codes by service:

```toml
# Banking codes
{ code = "*100#", menu = "banking", description = "Banking Services" },
{ code = "*101#", menu = "balance_check", description = "Quick Balance" },
{ code = "*102#", menu = "transfer", description = "Money Transfer" },

# Mobile codes
{ code = "*200#", menu = "mobile", description = "Mobile Services" },
{ code = "*201#", menu = "data_balance", description = "Data Balance" },
{ code = "*202#", menu = "data_packages", description = "Buy Data" },
```

### 3. Conditional Handling

```toml
# Handle only premium services
handle_codes = ["*999#", "*100#", "*200#"]
unrecognized_action = "forward"
unrecognized_message = "Service forwarded to network provider."
```

## Best Practices

1. **Use Memorable Codes**: Choose codes that are easy to remember
2. **Group Related Services**: Use similar prefixes for related services
3. **Provide Clear Descriptions**: Use meaningful descriptions for documentation
4. **Test Thoroughly**: Test all configured codes and edge cases
5. **Monitor Usage**: Log which codes are used most frequently
6. **Handle Errors Gracefully**: Configure appropriate error messages

## Troubleshooting

### Common Issues

1. **Code Not Working**: Check if it's in the `handle_codes` list
2. **Menu Not Found**: Verify the menu name exists in `[menus]`
3. **Session Issues**: Check session timeout settings
4. **Forwarding Problems**: Verify SMPP connection and binding

### Debug Mode

Enable debug logging to see how codes are processed:

```toml
[logging]
level = "debug"
debug = true
```

This will show detailed logs of USSD code processing, menu navigation, and session management.
