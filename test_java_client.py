#!/usr/bin/env python3
"""Simulate a Java SMPP client that binds to receive forwarded USSD requests"""

import socket
import struct
import binascii
import time
import threading

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

def parse_submit_sm(data):
    """Parse SUBMIT_SM PDU to extract message"""
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
        
        # Extract source_addr (MSISDN)
        msisdn_start = pos
        while pos < len(body) and body[pos] != 0:
            pos += 1
        msisdn = body[msisdn_start:pos].decode('ascii')
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
                return {'msisdn': msisdn, 'message': message}
        
        return None
    except Exception as e:
        print(f"Error parsing SUBMIT_SM: {e}")
        return None

def create_submit_sm_resp(sequence_number, status=0):
    """Create SUBMIT_SM_RESP PDU"""
    command_id = 0x80000004  # SUBMIT_SM_RESP
    
    if status == 0:
        # Success response with message ID
        message_id = f"MSG{int(time.time())}"
        body = message_id.encode('ascii') + b'\x00'
    else:
        # Error response
        body = b''
    
    command_length = 16 + len(body)
    header = struct.pack('>IIII', command_length, command_id, status, sequence_number)
    return header + body

def create_deliver_sm(msisdn, response_text, sequence_number):
    """Create DELIVER_SM PDU to send response back"""
    command_id = 0x00000005  # DELIVER_SM
    command_status = 0x00000000
    
    body = b''
    body += b'USSD\x00'  # service_type
    body += b'\x01\x01'   # source_addr_ton, source_addr_npi
    body += b'USSD\x00'   # source_addr
    body += b'\x01\x01'   # dest_addr_ton, dest_addr_npi
    body += msisdn.encode('ascii') + b'\x00'  # destination_addr
    body += b'\x40'       # esm_class (USSD)
    body += b'\x00' * 8   # other fields
    body += bytes([len(response_text)])  # message length
    body += response_text.encode('ascii')  # message
    
    command_length = 16 + len(body)
    header = struct.pack('>IIII', command_length, command_id, command_status, sequence_number)
    return header + body

def java_client_simulator():
    """Simulate a Java SMPP client that can receive forwarded requests"""
    try:
        print("🔗 Connecting to SMPP server as JavaClient...")
        
        # Connect to SMPP server
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.connect(('127.0.0.1', 2775))
        print("✅ Connected to SMPP server")
        
        # Bind as JavaClient (should be in forwarding_clients list)
        bind_pdu = create_bind_transceiver("JavaClient", "forward123")
        sock.send(bind_pdu)
        print("📤 Sent bind request as JavaClient")
        
        # Receive bind response
        bind_response = sock.recv(1024)
        header = parse_pdu_header(bind_response)
        if header and header['command_status'] == 0:
            print("✅ Bind successful - ready to receive forwarded requests")
        else:
            print(f"❌ Bind failed with status: {header['command_status'] if header else 'unknown'}")
            return
        
        sequence_counter = 2
        
        print("\n📡 Listening for forwarded USSD requests...")
        print("   (Test by sending a custom USSD code like *555# from another client)\n")
        
        while True:
            try:
                # Read PDU
                data = sock.recv(4096)
                if not data:
                    break
                
                header = parse_pdu_header(data)
                if not header:
                    continue
                
                print(f"📥 Received PDU: command_id=0x{header['command_id']:08x}, status={header['command_status']}")
                
                if header['command_id'] == 0x00000004:  # SUBMIT_SM (forwarded request)
                    parsed = parse_submit_sm(data)
                    if parsed:
                        msisdn = parsed['msisdn']
                        ussd_code = parsed['message']
                        
                        print(f"🎯 FORWARDED REQUEST: MSISDN={msisdn}, Code={ussd_code}")
                        
                        # Send SUBMIT_SM_RESP
                        submit_resp = create_submit_sm_resp(header['sequence_number'])
                        sock.send(submit_resp)
                        print("📤 Sent SUBMIT_SM_RESP")
                        
                        # Generate response based on the USSD code
                        if ussd_code == "*555#":
                            response = "🏦 Java Banking Service\n1. Account Info\n2. Transfer\n3. Exit"
                        elif ussd_code == "1":
                            response = "💰 Account: $1,500.00\nAvailable: $1,450.00\n\n0. Back"
                        elif ussd_code == "2":
                            response = "💸 Transfer Service\nEnter amount: \n\n0. Back"
                        elif ussd_code == "0":
                            response = "🏦 Java Banking Service\n1. Account Info\n2. Transfer\n3. Exit"
                        elif ussd_code == "3":
                            response = "Thank you for using Java Banking! 👋"
                        else:
                            response = f"❓ Unknown command: {ussd_code}\nPlease try again or dial 3 to exit."
                        
                        # Send DELIVER_SM with response
                        deliver_sm = create_deliver_sm(msisdn, response, sequence_counter)
                        sequence_counter += 1
                        sock.send(deliver_sm)
                        print(f"📤 Sent response: {response[:50]}...")
                        
                elif header['command_id'] == 0x80000005:  # DELIVER_SM_RESP
                    print("📥 Received DELIVER_SM_RESP")
                    
                elif header['command_id'] == 0x00000015:  # ENQUIRE_LINK
                    # Send ENQUIRE_LINK_RESP
                    enquire_resp = struct.pack('>IIII', 16, 0x80000015, 0, header['sequence_number'])
                    sock.send(enquire_resp)
                    print("📤 Sent ENQUIRE_LINK_RESP")
                
            except socket.timeout:
                continue
            except Exception as e:
                print(f"❌ Error processing PDU: {e}")
                break
        
        sock.close()
        print("🔌 Connection closed")
        
    except Exception as e:
        print(f"❌ Client error: {e}")

if __name__ == "__main__":
    print("🚀 Starting Java SMPP Client Simulator")
    print("   This simulates a Java client that can receive forwarded USSD requests")
    java_client_simulator()
