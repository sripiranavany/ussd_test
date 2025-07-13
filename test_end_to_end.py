#!/usr/bin/env python3
"""End-to-end test script that orchestrates the complete forwarding test"""

import subprocess
import time
import signal
import sys
import os
import socket
import struct

def check_server_running():
    """Check if the SMPP server is running"""
    try:
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.settimeout(1)
        result = sock.connect_ex(('127.0.0.1', 2775))
        sock.close()
        return result == 0
    except:
        return False

def create_submit_sm(msisdn, ussd_code, sequence_number):
    """Create a SUBMIT_SM PDU"""
    command_id = 0x00000004  # SUBMIT_SM
    command_status = 0x00000000
    
    body = b''
    body += b'USSD\x00'  # service_type
    body += b'\x01\x01'   # source_addr_ton, source_addr_npi
    body += msisdn.encode('ascii') + b'\x00'  # source_addr
    body += b'\x01\x01'   # dest_addr_ton, dest_addr_npi
    body += b'USSD\x00'   # destination_addr
    body += b'\x40'       # esm_class (USSD)
    body += b'\x00' * 8   # other fields
    body += bytes([len(ussd_code)])  # message length
    body += ussd_code.encode('ascii')  # message
    
    command_length = 16 + len(body)
    header = struct.pack('>IIII', command_length, command_id, command_status, sequence_number)
    return header + body

def send_ussd_request(ussd_code, msisdn="1234567890"):
    """Send a USSD request to test forwarding"""
    try:
        print(f"📱 Sending USSD request: {ussd_code} from {msisdn}")
        
        # Connect to server
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.connect(('127.0.0.1', 2775))
        
        # Bind as test client
        bind_pdu = struct.pack('>IIII', 16, 0x00000009, 0, 1)  # Basic bind
        sock.send(bind_pdu)
        
        # Wait for bind response
        bind_resp = sock.recv(1024)
        
        # Send SUBMIT_SM
        submit_sm = create_submit_sm(msisdn, ussd_code, 2)
        sock.send(submit_sm)
        
        # Wait for response
        response = sock.recv(1024)
        print(f"✅ Request sent, received response")
        
        sock.close()
        
    except Exception as e:
        print(f"❌ Error sending USSD request: {e}")

def run_test():
    """Run the complete end-to-end test"""
    print("🚀 Starting End-to-End Forwarding Test")
    print("=" * 60)
    
    # Step 1: Check if server is running
    if not check_server_running():
        print("❌ SMPP server is not running on port 2775")
        print("   Please run: cargo run --bin ussd_smpp_simulator")
        return False
    
    print("✅ SMPP server is running")
    
    # Step 2: Start Java client in background
    print("\n📱 Starting Java client simulator...")
    java_client_process = subprocess.Popen(
        [sys.executable, "test_java_client.py"],
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        bufsize=1
    )
    
    # Give the client time to connect
    time.sleep(2)
    
    # Step 3: Send test requests
    print("\n🔄 Sending test USSD requests...")
    
    test_cases = [
        "*555#",  # Should be forwarded to Java client
        "1",      # Follow-up request
        "2",      # Another follow-up
        "0",      # Back to menu
        "3",      # Exit
    ]
    
    for i, ussd_code in enumerate(test_cases):
        print(f"\n--- Test {i+1}: {ussd_code} ---")
        send_ussd_request(ussd_code)
        time.sleep(1)  # Wait between requests
    
    # Step 4: Clean up
    print("\n🛑 Stopping Java client...")
    java_client_process.terminate()
    
    try:
        java_client_process.wait(timeout=5)
    except subprocess.TimeoutExpired:
        java_client_process.kill()
    
    print("✅ Test completed!")
    print("\nCheck the Java client output above to verify that:")
    print("1. Java client connected successfully")
    print("2. Custom USSD codes (*555#) were forwarded")
    print("3. Follow-up requests were handled")
    print("4. Responses were sent back to the server")

if __name__ == "__main__":
    run_test()
