#!/usr/bin/env python3
import socket
import struct
import time

def create_smpp_bind_pdu(system_id, password):
    """Create SMPP BIND_TRANSCEIVER PDU"""
    BIND_TRANSCEIVER = 0x00000009
    
    # Create body
    body = system_id.encode('ascii') + b'\x00'  # system_id
    body += password.encode('ascii') + b'\x00'  # password
    body += b'\x00'  # system_type
    body += b'\x34'  # interface_version
    body += b'\x00'  # addr_ton
    body += b'\x00'  # addr_npi
    body += b'\x00'  # address_range
    
    # Create header
    length = 16 + len(body)
    header = struct.pack('>IIII', length, BIND_TRANSCEIVER, 0, 1)
    
    return header + body

def create_submit_sm_pdu(from_msisdn, to_msisdn, message, sequence_num):
    """Create SMPP SUBMIT_SM PDU for USSD"""
    SUBMIT_SM = 0x00000004
    
    # Create body
    body = b'USSD\x00'  # service_type
    body += b'\x01'  # source_addr_ton
    body += b'\x01'  # source_addr_npi
    body += from_msisdn.encode('ascii') + b'\x00'  # source_addr
    body += b'\x01'  # dest_addr_ton
    body += b'\x01'  # dest_addr_npi
    body += to_msisdn.encode('ascii') + b'\x00'  # destination_addr
    body += b'\x00'  # esm_class
    body += b'\x00'  # protocol_id
    body += b'\x00'  # priority_flag
    body += b'\x00'  # schedule_delivery_time
    body += b'\x00'  # validity_period
    body += b'\x00'  # registered_delivery
    body += b'\x00'  # replace_if_present_flag
    body += b'\x00'  # data_coding
    body += b'\x00'  # sm_default_msg_id
    body += bytes([len(message)])  # sm_length
    body += message.encode('ascii')  # short_message
    
    # Create header
    length = 16 + len(body)
    header = struct.pack('>IIII', length, SUBMIT_SM, 0, sequence_num)
    
    return header + body

def test_custom_ussd():
    """Test custom USSD code forwarding"""
    try:
        # Connect to SMPP server
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.connect(('127.0.0.1', 2775))
        print("Connected to SMMP server")
        
        # Send BIND_TRANSCEIVER
        bind_pdu = create_smpp_bind_pdu("TestClient", "testpass")
        sock.send(bind_pdu)
        
        # Read bind response
        response = sock.recv(1024)
        print(f"Bind response: {response.hex()}")
        
        # Send custom USSD code
        submit_pdu = create_submit_sm_pdu("1234567890", "123", "*555#", 2)
        sock.send(submit_pdu)
        print("Sent custom USSD code: *555#")
        
        # Read submit response
        response = sock.recv(1024)
        print(f"Submit response: {response.hex()}")
        
        # Read USSD response
        response = sock.recv(1024)
        print(f"USSD response: {response.hex()}")
        
        # Try to extract text from response
        if len(response) > 50:  # rough check for valid response
            text_start = response.find(b'Welcome')
            if text_start == -1:
                text_start = response.find(b'Unknown')
            if text_start != -1:
                text_end = len(response)
                text = response[text_start:text_end].decode('ascii', errors='ignore')
                print(f"Response text: {text}")
        
        sock.close()
        
    except Exception as e:
        print(f"Error: {e}")

if __name__ == "__main__":
    test_custom_ussd()
