#!/usr/bin/env python3
"""
Test script for the Rust USSD SMPP Client Simulator
This script tests the complete forwarding functionality with the Rust client.
"""

import subprocess
import time
import signal
import sys
import os
import socket
import struct
import threading
import json

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
        
        # Send bind_transceiver
        bind_pdu = struct.pack('>IIII', 16, 0x00000009, 0x00000000, 1)
        sock.send(bind_pdu)
        
        # Read bind response
        response = sock.recv(1024)
        print(f"📡 Bind response received: {len(response)} bytes")
        
        # Send USSD request
        submit_pdu = create_submit_sm(msisdn, ussd_code, 2)
        sock.send(submit_pdu)
        
        # Read submit response
        response = sock.recv(1024)
        print(f"📡 Submit response received: {len(response)} bytes")
        
        # Read deliver_sm (response from client)
        response = sock.recv(1024)
        if len(response) > 16:
            # Parse the message
            message_length = response[38]
            message = response[39:39+message_length].decode('ascii')
            print(f"📥 Received response: {message}")
            return message
        
        sock.close()
        return None
        
    except Exception as e:
        print(f"❌ Error sending USSD request: {e}")
        return None

def test_menu_navigation():
    """Test basic menu navigation"""
    print("\n🧪 Testing Menu Navigation")
    print("=" * 50)
    
    # Test default/main menu
    response = send_ussd_request("*999#")
    if response and "Custom Services Menu" in response:
        print("✅ Main menu test passed")
    else:
        print("❌ Main menu test failed")
        return False
    
    # Test banking submenu
    response = send_ussd_request("*999*1#")
    if response and "Banking Services" in response:
        print("✅ Banking submenu test passed")
    else:
        print("❌ Banking submenu test failed")
        return False
    
    # Test balance check
    response = send_ussd_request("*999*1*1#")
    if response and "balance" in response.lower():
        print("✅ Balance check test passed")
    else:
        print("❌ Balance check test failed")
        return False
    
    return True

def test_custom_responses():
    """Test custom response functionality"""
    print("\n🧪 Testing Custom Responses")
    print("=" * 50)
    
    # Test mobile services
    response = send_ussd_request("*999*2#")
    if response and "Mobile Services" in response:
        print("✅ Mobile services menu test passed")
    else:
        print("❌ Mobile services menu test failed")
        return False
    
    # Test data balance
    response = send_ussd_request("*999*2*1#")
    if response and "data" in response.lower():
        print("✅ Data balance test passed")
    else:
        print("❌ Data balance test failed")
        return False
    
    return True

def test_error_handling():
    """Test error handling with invalid inputs"""
    print("\n🧪 Testing Error Handling")
    print("=" * 50)
    
    # Test invalid menu option
    response = send_ussd_request("*999*9#")
    if response and ("invalid" in response.lower() or "error" in response.lower()):
        print("✅ Invalid option handling test passed")
    else:
        print("❌ Invalid option handling test failed")
        return False
    
    # Test unrecognized USSD code
    response = send_ussd_request("*123#")
    if response:
        print("✅ Unrecognized USSD code handling test passed")
    else:
        print("❌ Unrecognized USSD code handling test failed")
        return False
    
    return True

def start_rust_client():
    """Start the Rust client simulator"""
    print("🦀 Starting Rust USSD Client Simulator...")
    
    # Change to client directory
    client_dir = "/sripiranavan/development/learn/rust/demo/ussd_smpp_client_simulator"
    
    # Start client process
    process = subprocess.Popen(
        ["cargo", "run", "--"],
        cwd=client_dir,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True
    )
    
    # Wait a bit for client to start
    time.sleep(3)
    
    # Check if process is running
    if process.poll() is None:
        print("✅ Rust client started successfully")
        return process
    else:
        stdout, stderr = process.communicate()
        print(f"❌ Failed to start Rust client:")
        print(f"STDOUT: {stdout}")
        print(f"STDERR: {stderr}")
        return None

def start_server():
    """Start the SMPP server"""
    print("🖥️  Starting SMPP Server...")
    
    server_dir = "/sripiranavan/development/learn/rust/demo/ussd_smpp_simulator"
    
    # Start server process
    process = subprocess.Popen(
        ["cargo", "run", "--"],
        cwd=server_dir,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True
    )
    
    # Wait for server to start
    print("⏳ Waiting for server to start...")
    for i in range(10):
        if check_server_running():
            print("✅ Server is running on port 2775")
            return process
        time.sleep(1)
    
    print("❌ Server failed to start")
    return None

def main():
    """Main test function"""
    print("🚀 USSD SMPP Client Simulator Test Suite")
    print("=" * 60)
    
    # Start server
    server_process = start_server()
    if not server_process:
        print("❌ Cannot start server. Exiting.")
        return False
    
    try:
        # Start client
        client_process = start_rust_client()
        if not client_process:
            print("❌ Cannot start client. Exiting.")
            return False
        
        try:
            # Wait a bit for client to connect
            time.sleep(2)
            
            # Run tests
            tests_passed = 0
            total_tests = 3
            
            if test_menu_navigation():
                tests_passed += 1
            
            if test_custom_responses():
                tests_passed += 1
            
            if test_error_handling():
                tests_passed += 1
            
            # Print results
            print(f"\n📊 Test Results")
            print("=" * 50)
            print(f"✅ Tests passed: {tests_passed}/{total_tests}")
            print(f"❌ Tests failed: {total_tests - tests_passed}/{total_tests}")
            
            if tests_passed == total_tests:
                print("🎉 All tests passed!")
                return True
            else:
                print("⚠️  Some tests failed.")
                return False
                
        finally:
            # Clean up client
            if client_process:
                print("🧹 Stopping client...")
                client_process.terminate()
                client_process.wait()
                
    finally:
        # Clean up server
        if server_process:
            print("🧹 Stopping server...")
            server_process.terminate()
            server_process.wait()

if __name__ == "__main__":
    success = main()
    sys.exit(0 if success else 1)
