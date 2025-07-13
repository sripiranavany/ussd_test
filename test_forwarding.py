#!/usr/bin/env python3
"""Test script to send a custom USSD code and verify forwarding to Java client"""

import socket
import struct
import binascii
import time

def create_bind_transceiver(system_id, password):
    """Create a bind_transceiver PDU"""
    command_id = 0x00000009  # BIND_TRANSCEIVER
    command_status = 0x00000000
    sequence_number = 0x00000001
    
    system_id_bytes = system_id.encode('ascii') + b'\x00'
    password_bytes = password.encode('ascii') + b'\x00'
    system_type = b'\x00'
    interface_version = b'\x34'
    addr_ton = b'\x00'
    addr_npi = b'\x00'
    address_range = b'\x00'
    
    body = system_id_bytes + password_bytes + system_type + interface_version + addr_ton + addr_npi + address_range
    command_length = 16 + len(body)
    
    header = struct.pack('>IIII', command_length, command_id, command_status, sequence_number)
    return header + body

def create_submit_sm(msisdn, ussd_code, sequence_number):
    """Create SUBMIT_SM PDU"""
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

def parse_pdu_header(data):
    """Parse SMPP PDU header"""
    if len(data) < 16:
        return None
    
    header = struct.unpack('>IIII', data[:16])
    return {
        'command_length': header[0],
        'command_id': header[1],
        'command_status': header[2],
        'sequence_number': header[3]
    }

def parse_deliver_sm(data):
    """Parse DELIVER_SM PDU to extract response"""
    if len(data) < 16:
        return None
    
    body = data[16:]
    try:
        pos = 0
        
        # Skip service_type (null-terminated)
        while pos < len(body) and body[pos] != 0:
            pos += 1
        pos += 1
        
        # Skip source_addr_ton, source_addr_npi
        pos += 2
        
        # Skip source_addr (null-terminated)
        while pos < len(body) and body[pos] != 0:
            pos += 1
        pos += 1
        
        # Skip dest_addr_ton, dest_addr_npi
        pos += 2
        
        # Skip destination_addr (null-terminated)
        while pos < len(body) and body[pos] != 0:
            pos += 1
        pos += 1
        
        # Skip other fields to get to message
        pos += 12  # Skip esm_class through sm_default_msg_id
        
        # Get message length and content
        if pos < len(body):
            msg_length = body[pos]
            pos += 1
            if pos + msg_length <= len(body):
                message = body[pos:pos + msg_length].decode('ascii')
                return message
        
        return None
    except Exception as e:
        print(f"Error parsing DELIVER_SM: {e}")
        return None

def test_forwarding():
    """Test sending a custom USSD code that should be forwarded to Java client"""
    try:
        print("🧪 Testing USSD forwarding to Java client...")
        
        # Connect to SMPP server
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.connect(('127.0.0.1', 2775))
        print("✅ Connected to SMPP server")
        
        # Bind as test client
        bind_pdu = create_bind_transceiver("TestClient", "testpass")
        sock.send(bind_pdu)
        print("📤 Sent bind request as TestClient")
        
        # Receive bind response
        bind_response = sock.recv(1024)
        header = parse_pdu_header(bind_response)
        if header and header['command_status'] == 0:
            print("✅ Bind successful")
        else:
            print(f"❌ Bind failed with status: {header['command_status'] if header else 'unknown'}")
            return
        
        # Send USSD request for *555# (should be forwarded to Java client)
        print(f"\n📱 Sending custom USSD code: *555#")
        submit_pdu = create_submit_sm("1234567890", "*555#", 2)
        sock.send(submit_pdu)
        print("📤 Sent SUBMIT_SM for *555#")
        
        # Wait for responses
        timeout_count = 0
        while timeout_count < 10:  # Wait up to 10 seconds
            try:
                sock.settimeout(1.0)
                data = sock.recv(4096)
                if not data:
                    break
                
                header = parse_pdu_header(data)
                if not header:
                    continue
                
                print(f"📥 Received PDU: command_id=0x{header['command_id']:08x}, status={header['command_status']}")
                
                if header['command_id'] == 0x80000004:  # SUBMIT_SM_RESP
                    print("📥 Received SUBMIT_SM_RESP")
                    
                elif header['command_id'] == 0x00000005:  # DELIVER_SM (response from Java client)
                    response = parse_deliver_sm(data)
                    if response:
                        print(f"🎯 RESPONSE FROM JAVA CLIENT: {response}")
                        
                        # Send DELIVER_SM_RESP
                        deliver_resp = struct.pack('>IIII', 16, 0x80000005, 0, header['sequence_number'])
                        sock.send(deliver_resp)
                        print("📤 Sent DELIVER_SM_RESP")
                        
                        # Test menu navigation
                        if "1. Account Info" in response:
                            print(f"\n📱 Navigating to option 1...")
                            submit_pdu = create_submit_sm("1234567890", "1", 3)
                            sock.send(submit_pdu)
                            print("📤 Sent SUBMIT_SM for option '1'")
                        
                elif header['command_id'] == 0x00000015:  # ENQUIRE_LINK
                    # Send ENQUIRE_LINK_RESP
                    enquire_resp = struct.pack('>IIII', 16, 0x80000015, 0, header['sequence_number'])
                    sock.send(enquire_resp)
                    print("📤 Sent ENQUIRE_LINK_RESP")
                    
                timeout_count = 0  # Reset timeout counter
                
            except socket.timeout:
                timeout_count += 1
                continue
        
        print("\n✅ Test completed successfully!")
        sock.close()
        
    except Exception as e:
        print(f"❌ Test error: {e}")

if __name__ == "__main__":
    test_forwarding()
