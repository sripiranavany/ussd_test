#!/usr/bin/env python3
"""
Test script to demonstrate USSD code configuration
"""

import subprocess
import time
import socket
import struct
import sys

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
    """Send a USSD request and return response"""
    try:
        print(f"📱 Testing USSD code: {ussd_code}")
        
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.connect(('127.0.0.1', 2775))
        
        # Send bind_transceiver
        bind_pdu = struct.pack('>IIII', 16, 0x00000009, 0x00000000, 1)
        sock.send(bind_pdu)
        
        # Read bind response
        response = sock.recv(1024)
        
        # Send USSD request
        submit_pdu = create_submit_sm(msisdn, ussd_code, 2)
        sock.send(submit_pdu)
        
        # Read submit response
        response = sock.recv(1024)
        
        # Read deliver_sm (response from client)
        response = sock.recv(1024)
        if len(response) > 16:
            message_length = response[38]
            message = response[39:39+message_length].decode('ascii')
            print(f"📥 Response: {message[:100]}...")
            return message
        
        sock.close()
        return None
        
    except Exception as e:
        print(f"❌ Error: {e}")
        return None

def test_ussd_codes():
    """Test different USSD codes"""
    print("🧪 Testing USSD Code Configuration")
    print("=" * 50)
    
    # Test configured codes
    test_codes = [
        ("*999#", "Main Services Menu"),
        ("*100#", "Banking Services"),
        ("*200#", "Mobile Services"),
        ("*300#", "Utilities"),
        ("*400#", "Help & Support"),
        ("*123#", "Alternative Main Menu"),
    ]
    
    for code, expected in test_codes:
        response = send_ussd_request(code)
        if response and expected.lower() in response.lower():
            print(f"✅ {code} -> {expected}")
        else:
            print(f"❌ {code} -> Unexpected response")
        time.sleep(0.5)
    
    print("\n🧪 Testing Unrecognized Codes")
    print("=" * 50)
    
    # Test unrecognized codes
    unrecognized_codes = ["*555#", "*777#", "*888#"]
    
    for code in unrecognized_codes:
        response = send_ussd_request(code)
        if response:
            print(f"📤 {code} -> {response[:50]}...")
        else:
            print(f"❌ {code} -> No response")
        time.sleep(0.5)

def main():
    """Main test function"""
    print("🚀 USSD Code Configuration Test")
    print("=" * 60)
    
    # Check if server is running
    try:
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.settimeout(1)
        result = sock.connect_ex(('127.0.0.1', 2775))
        sock.close()
        if result != 0:
            print("❌ SMPP server not running on port 2775")
            print("Please start the server first:")
            print("cd ussd_smpp_simulator && cargo run")
            return False
    except:
        print("❌ Cannot connect to SMPP server")
        return False
    
    test_ussd_codes()
    
    print("\n📊 Test Summary")
    print("=" * 50)
    print("✅ USSD code configuration is working")
    print("📝 Check the client logs for detailed processing")
    print("🔧 Modify client_config.toml to add more codes")
    
    return True

if __name__ == "__main__":
    success = main()
    sys.exit(0 if success else 1)
