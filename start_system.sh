#!/bin/bash
# Start the USSD SMPP Simulator components in separate terminals

echo "🚀 Starting USSD SMPP Simulator System"
echo "======================================"

# Function to check if a port is in use
check_port() {
    local port=$1
    if lsof -i :$port >/dev/null 2>&1; then
        echo "✅ Port $port is in use"
        return 0
    else
        echo "❌ Port $port is not in use"
        return 1
    fi
}

# Kill any existing processes
echo "🧹 Cleaning up existing processes..."
pkill -f "ussd_smpp_simulator" 2>/dev/null || true
pkill -f "test_java_client.py" 2>/dev/null || true
sleep 2

# Start the server in a new terminal
echo "🖥️  Starting SMPP Server..."
if command -v gnome-terminal >/dev/null 2>&1; then
    gnome-terminal --title="SMPP Server" --working-directory="$(pwd)/ussd_smpp_simulator" -- bash -c "
        echo '🖥️  SMPP Server Starting...'
        cargo run --bin ussd_smpp_simulator
        echo 'Press any key to exit...'
        read -n 1
    " &
elif command -v xterm >/dev/null 2>&1; then
    xterm -title "SMPP Server" -e "cd $(pwd)/ussd_smpp_simulator && cargo run --bin ussd_smpp_simulator; echo 'Press any key to exit...'; read -n 1" &
else
    echo "❌ No terminal emulator found. Starting server in background..."
    cd ussd_smpp_simulator
    cargo run --bin ussd_smpp_simulator &
    SERVER_PID=$!
    cd ..
fi

# Wait for server to start
echo "⏳ Waiting for server to start..."
for i in {1..10}; do
    if check_port 2775; then
        echo "✅ Server is running on port 2775"
        break
    fi
    echo "   Waiting... ($i/10)"
    sleep 2
done

if ! check_port 2775; then
    echo "❌ Server failed to start"
    exit 1
fi

# Start the Java client in a new terminal
echo "📱 Starting Java Client Simulator..."
if command -v gnome-terminal >/dev/null 2>&1; then
    gnome-terminal --title="Java Client" --working-directory="$(pwd)" -- bash -c "
        echo '📱 Java Client Starting...'
        python3 test_java_client.py
        echo 'Press any key to exit...'
        read -n 1
    " &
elif command -v xterm >/dev/null 2>&1; then
    xterm -title "Java Client" -e "cd $(pwd) && python3 test_java_client.py; echo 'Press any key to exit...'; read -n 1" &
else
    echo "❌ No terminal emulator found. Starting client in background..."
    python3 test_java_client.py &
    CLIENT_PID=$!
fi

# Wait for client to connect
echo "⏳ Waiting for client to connect..."
sleep 3

# Start the test in a new terminal
echo "🧪 Starting Test Client..."
if command -v gnome-terminal >/dev/null 2>&1; then
    gnome-terminal --title="Test Client" --working-directory="$(pwd)" -- bash -c "
        echo '🧪 Test Client Starting...'
        echo 'You can now send USSD requests manually:'
        echo 'python3 -c \"
import socket, struct
def send_ussd(code):
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    s.connect(('127.0.0.1', 2775))
    # Send bind
    s.send(struct.pack('>IIII', 16, 0x00000009, 0, 1))
    s.recv(1024)
    # Send SUBMIT_SM
    body = b'USSD\\x00\\x01\\x011234567890\\x00\\x01\\x01USSD\\x00\\x40' + b'\\x00'*8 + bytes([len(code)]) + code.encode()
    s.send(struct.pack('>IIII', 16+len(body), 0x00000004, 0, 2) + body)
    s.recv(1024)
    s.close()
    print(f'Sent: {code}')

# Test the forwarding
print('Testing forwarding with *555#...')
send_ussd('*555#')
\"'
        echo
        echo 'Or run the comprehensive test:'
        echo 'python3 test_end_to_end.py'
        echo
        echo 'Press any key to exit...'
        read -n 1
    " &
elif command -v xterm >/dev/null 2>&1; then
    xterm -title "Test Client" -e "cd $(pwd) && echo 'Test environment ready. Run: python3 test_end_to_end.py'; bash" &
else
    echo "📋 Manual test commands:"
    echo "   python3 test_end_to_end.py"
fi

echo
echo "✅ System started! Components running:"
echo "   🖥️  SMPP Server: http://localhost:2775"
echo "   📱 Java Client: Connected and listening"
echo "   🧪 Test Client: Ready to send requests"
echo
echo "💡 Test the forwarding by running in a new terminal:"
echo "   python3 test_end_to_end.py"
echo
echo "🛑 To stop all components:"
echo "   pkill -f 'ussd_smpp_simulator'"
echo "   pkill -f 'test_java_client.py'"
echo
echo "Press Ctrl+C to exit this script..."

# Wait for user to exit
trap 'echo "🛑 Stopping..."; pkill -f "ussd_smpp_simulator" 2>/dev/null || true; pkill -f "test_java_client.py" 2>/dev/null || true; exit 0' INT
while true; do
    sleep 1
done
