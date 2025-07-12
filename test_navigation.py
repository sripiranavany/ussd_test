#!/usr/bin/env python3
"""Test navigation flow for custom USSD service"""

import socket
import struct
import binascii
import time

def create_bind_transceiver(system_id, password):
    """Create a bind_transceiver PDU"""
    command_id = 0x00000009  # BIND_TRANSCEIVER
    command_status = 0x00000000
    sequence_number = 0x00000001
    
    # Body
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

def create_submit_sm(source_addr, destination_addr, short_message, sequence_number):
    """Create a submit_sm PDU"""
    command_id = 0x00000004  # SUBMIT_SM
    command_status = 0x00000000
    
    # Body
    service_type = b'\x00'
    source_addr_ton = b'\x01'
    source_addr_npi = b'\x01'
    source_addr_bytes = source_addr.encode('ascii') + b'\x00'
    dest_addr_ton = b'\x00'
    dest_addr_npi = b'\x00'
    destination_addr_bytes = destination_addr.encode('ascii') + b'\x00'
    esm_class = b'\x40'  # USSD indication
    protocol_id = b'\x00'
    priority_flag = b'\x00'
    schedule_delivery_time = b'\x00'
    validity_period = b'\x00'
    registered_delivery = b'\x00'
    replace_if_present_flag = b'\x00'
    data_coding = b'\x00'
    sm_default_msg_id = b'\x00'
    sm_length = bytes([len(short_message)])
    short_message_bytes = short_message.encode('ascii')
    
    body = (service_type + source_addr_ton + source_addr_npi + source_addr_bytes +
            dest_addr_ton + dest_addr_npi + destination_addr_bytes + esm_class +
            protocol_id + priority_flag + schedule_delivery_time + validity_period +
            registered_delivery + replace_if_present_flag + data_coding +
            sm_default_msg_id + sm_length + short_message_bytes)
    
    command_length = 16 + len(body)
    header = struct.pack('>IIII', command_length, command_id, command_status, sequence_number)
    return header + body

def parse_deliver_sm(data):
    """Parse DELIVER_SM PDU and extract message text"""
    if len(data) < 16:
        return "Invalid PDU"
    
    # Skip header
    body = data[16:]
    
    # Parse body to get to the message
    try:
        pos = 0
        # Skip service_type (null-terminated)
        while pos < len(body) and body[pos] != 0:
            pos += 1
        pos += 1  # Skip null terminator
        
        # Skip source_addr_ton, source_addr_npi
        pos += 2
        
        # Skip source_addr (null-terminated)
        while pos < len(body) and body[pos] != 0:
            pos += 1
        pos += 1  # Skip null terminator
        
        # Skip dest_addr_ton, dest_addr_npi
        pos += 2
        
        # Skip destination_addr (null-terminated)
        while pos < len(body) and body[pos] != 0:
            pos += 1
        pos += 1  # Skip null terminator
        
        # Skip esm_class, protocol_id, priority_flag
        pos += 3
        
        # Skip schedule_delivery_time (null-terminated)
        while pos < len(body) and body[pos] != 0:
            pos += 1
        pos += 1  # Skip null terminator
        
        # Skip validity_period (null-terminated)
        while pos < len(body) and body[pos] != 0:
            pos += 1
        pos += 1  # Skip null terminator
        
        # Skip registered_delivery, replace_if_present_flag, data_coding, sm_default_msg_id
        pos += 4
        
        # Get sm_length
        if pos < len(body):
            sm_length = body[pos]
            pos += 1
            
            # Extract message
            if pos + sm_length <= len(body):
                message = body[pos:pos + sm_length].decode('ascii', errors='ignore')
                return message
        
        return "Could not parse message"
    except Exception as e:
        return f"Parse error: {e}"

def test_navigation():
    """Test USSD navigation flow"""
    try:
        # Connect to SMPP server
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.connect(('127.0.0.1', 2775))
        print("Connected to SMMP server")
        
        # Bind
        bind_pdu = create_bind_transceiver("USSDTestClient", "testpass123")
        sock.send(bind_pdu)
        
        # Receive bind response
        response = sock.recv(1024)
        print(f"Bind response: {binascii.hexlify(response).decode()}")
        
        # Test navigation flow
        test_steps = [
            ("*555#", "Initial menu"),
            ("2", "Transfer Money option"),
            ("0", "Back to main menu"),
            ("#", "Exit service")
        ]
        
        for step, description in test_steps:
            print(f"\n--- {description} ---")
            print(f"Sending: {step}")
            
            # Send USSD request
            submit_pdu = create_submit_sm("1234567890", "123", step, 2)
            sock.send(submit_pdu)
            
            # Receive submit response
            submit_response = sock.recv(1024)
            print(f"Submit response: {binascii.hexlify(submit_response).decode()}")
            
            # Receive DELIVER_SM with response
            deliver_sm = sock.recv(1024)
            print(f"USSD response: {binascii.hexlify(deliver_sm).decode()}")
            
            # Parse and display response text
            response_text = parse_deliver_sm(deliver_sm)
            print(f"Response text: {response_text}")
            
            # Send DELIVER_SM_RESP
            deliver_resp_header = struct.pack('>IIII', 16, 0x80000005, 0, 2)
            sock.send(deliver_resp_header)
            
            time.sleep(1)  # Brief pause between steps
            
            # If session ended, break
            if "Thank you" in response_text:
                print("Session ended")
                break
        
        sock.close()
        print("Test completed successfully")
        
    except Exception as e:
        print(f"Error: {e}")

if __name__ == "__main__":
    test_navigation()
