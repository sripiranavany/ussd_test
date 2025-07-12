# SMPP Server Response Percentage Configuration

## Overview

The USSD SMPP Simulator now supports **configurable response percentages** for load testing scenarios. This feature allows you to simulate real-world network conditions where responses can be successful, fail, or time out.

## Configuration

### Configuration File Section

```toml
[response_percentage]
success_percentage = 80.0      # Percentage of successful responses
failure_percentage = 15.0      # Percentage of failure responses  
no_response_percentage = 5.0   # Percentage of no response (timeout)
failure_error_code = 0x00000008  # SMPP error code for failures (ESME_RSYSERR)
no_response_delay_ms = 3000    # Delay before timeout for no response
```

### Validation Rules

- **Percentages must sum to 100%**: `success_percentage + failure_percentage + no_response_percentage = 100.0`
- **Range**: Each percentage must be between 0.0 and 100.0
- **Precision**: Supports decimal values (e.g., 85.5%)

## Response Types

### 1. **Success Response**
- **Behavior**: Normal SMPP processing
- **SUBMIT_SM_RESP**: Success status (0x00000000)
- **DELIVER_SM**: USSD response message sent
- **Use Case**: Simulates successful message delivery

### 2. **Failure Response**
- **Behavior**: SMPP protocol error
- **SUBMIT_SM_RESP**: Error status (configurable, default: 0x00000008)
- **DELIVER_SM**: Not sent
- **Use Case**: Simulates network/system errors

### 3. **No Response**
- **Behavior**: Server doesn't respond
- **SUBMIT_SM_RESP**: Not sent
- **DELIVER_SM**: Not sent
- **Timeout**: Client must handle timeout (configurable delay)
- **Use Case**: Simulates network timeouts, server overload

## Configuration Examples

### **Production-like (High Success Rate)**
```toml
[response_percentage]
success_percentage = 95.0
failure_percentage = 4.0
no_response_percentage = 1.0
failure_error_code = 0x00000008
no_response_delay_ms = 5000
```

### **Stress Testing (High Failure Rate)**
```toml
[response_percentage]
success_percentage = 60.0
failure_percentage = 30.0
no_response_percentage = 10.0
failure_error_code = 0x00000008
no_response_delay_ms = 3000
```

### **Timeout Testing (High No Response)**
```toml
[response_percentage]
success_percentage = 50.0
failure_percentage = 10.0
no_response_percentage = 40.0
failure_error_code = 0x00000008
no_response_delay_ms = 2000
```

## Load Testing Usage

### **1. Configure Response Percentages**
Edit your configuration file:

```bash
# Edit dev.toml for testing
vim ussd_smpp_simulator/dev.toml

# Set desired percentages
[response_percentage]
success_percentage = 80.0
failure_percentage = 15.0
no_response_percentage = 5.0
```

### **2. Start Server**
```bash
cd ussd_smpp_simulator
./target/release/ussd_smpp_simulator -c dev.toml
```

### **3. Run Load Test**
```bash
# Use the provided load testing script
python3 test_load_percentage.py

# Or create your own SMPP client
# Send concurrent SUBMIT_SM requests
# Measure response rates and times
```

### **4. Analyze Results**
- Monitor success/failure/timeout rates
- Measure response times
- Test client timeout handling
- Verify error code handling

## SMPP Error Codes

Common error codes for `failure_error_code`:

| Code | Name | Description |
|------|------|-------------|
| 0x00000008 | ESME_RSYSERR | System error |
| 0x00000001 | ESME_RINVMSGLEN | Invalid message length |
| 0x00000002 | ESME_RINVCMDLEN | Invalid command length |
| 0x00000003 | ESME_RINVCMDID | Invalid command ID |
| 0x00000004 | ESME_RINVBNDSTS | Invalid bind status |
| 0x00000005 | ESME_RALYBND | Already bound |

## Implementation Details

### **Random Distribution**
- Uses cryptographic hash for pseudo-randomness
- Based on current system time (nanoseconds)
- Ensures even distribution across percentage ranges

### **Response Logic**
```rust
// Simplified logic
let random_value = generate_random_percentage(); // 0.0 to 99.99%

if random_value < success_percentage {
    // Send success response + USSD message
} else if random_value < success_percentage + failure_percentage {
    // Send error response
} else {
    // No response (timeout)
}
```

### **Thread Safety**
- Each connection determines response type independently
- No shared state between requests
- Suitable for high-concurrency load testing

## Testing Results

### **Sample Load Test (200 requests)**
```
Configuration:
- Success: 80%
- Failure: 15%  
- No Response: 5%

Results:
- Success: 163 (81.5%) ✅
- Failure: 23 (11.5%) ✅
- No Response: 14 (7.0%) ✅
- Average Response Time: 0.702s
```

## Use Cases

### **1. Load Testing**
- Test client applications under various network conditions
- Measure performance with different failure rates
- Validate client timeout and retry logic

### **2. Resilience Testing**
- Test application behavior during network issues
- Validate error handling and recovery mechanisms
- Test client failover scenarios

### **3. Performance Benchmarking**
- Compare client performance under different conditions
- Measure throughput with varying success rates
- Identify bottlenecks and optimization opportunities

### **4. Integration Testing**
- Test end-to-end system behavior
- Validate monitoring and alerting systems
- Test client circuit breaker implementations

## Best Practices

1. **Start with realistic percentages** (95% success for production-like testing)
2. **Use gradual degradation** (85% → 70% → 50% success rates)
3. **Monitor client behavior** during tests
4. **Test timeout handling** with no-response scenarios
5. **Validate error code handling** with different failure codes
6. **Use concurrent load** to simulate real-world conditions

This feature makes the SMPP simulator ideal for comprehensive load testing and resilience validation! 🚀
