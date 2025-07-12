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
