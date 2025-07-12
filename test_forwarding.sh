#!/bin/bash

# Test script to verify USSD forwarding functionality

echo "=== USSD Forwarding Test ==="
echo ""

# Start the client simulator in forwarding mode
echo "Starting client simulator in forwarding mode..."
cd /sripiranavan/development/learn/rust/demo/ussd_client_simulator
cargo run forwarding > forwarding_test.log 2>&1 &
FORWARDING_PID=$!

# Give it time to start
sleep 2

# Start the SMPP server
echo "Starting SMPP server..."
cd /sripiranavan/development/learn/rust/demo/ussd_smpp_simulator
cargo run > server_test.log 2>&1 &
SERVER_PID=$!

# Give it time to start
sleep 2

echo "Testing forwarding with custom USSD code *777#"

# Test forwarding with a custom code
cd /sripiranavan/development/learn/rust/demo/ussd_client_simulator
echo "*777#" | timeout 10 cargo run client 1234567890 > test_result.log 2>&1

echo "Test completed. Results:"
if grep -q "Welcome to Custom USSD Service" test_result.log; then
    echo "✓ Forwarding test PASSED"
else
    echo "✗ Forwarding test FAILED"
    echo "Response was:"
    cat test_result.log
fi

# Clean up
echo "Cleaning up..."
kill $FORWARDING_PID 2>/dev/null
kill $SERVER_PID 2>/dev/null
wait $FORWARDING_PID 2>/dev/null
wait $SERVER_PID 2>/dev/null

echo "=== Test Complete ==="
