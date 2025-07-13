#!/usr/bin/env python3
"""Simple USSD test client for manual testing"""

import socket
import struct
import time

def create_bind_transceiver(system_id="TestClient", password="test123"):
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
    """Send a USSD request and wait for response"""
    try:
        print(f"📱 Connecting to server...")
        
        # Connect to server
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.connect(('127.0.0.1', 2775))
        print("✅ Connected")
        
        # Bind
        bind_pdu = create_bind_transceiver()
        sock.send(bind_pdu)
        bind_resp = sock.recv(1024)
        print("✅ Bind successful")
        
        # Send SUBMIT_SM
        print(f"📤 Sending USSD: {ussd_code} from {msisdn}")
        submit_sm = create_submit_sm(msisdn, ussd_code, 2)
        sock.send(submit_sm)
        
        # Wait for SUBMIT_SM_RESP
        submit_resp = sock.recv(1024)
        print("✅ SUBMIT_SM_RESP received")
        
        # Wait for DELIVER_SM (response)
        sock.settimeout(5)
        try:
            deliver_sm = sock.recv(1024)
            if len(deliver_sm) > 16:
                print("📥 Response received from server")
                # Parse the response message (simplified)
                body = deliver_sm[16:]
                # Skip to message part (this is a simplified parser)
                try:
                    msg_start = body.find(b'\x00', 20)  # Find end of addresses
                    if msg_start > 0:
                        msg_len_pos = msg_start + 10  # Approximate position
                        if msg_len_pos < len(body):
                            msg_len = body[msg_len_pos]
                            if msg_len_pos + 1 + msg_len <= len(body):
                                message = body[msg_len_pos + 1:msg_len_pos + 1 + msg_len].decode('ascii', errors='ignore')
                                print(f"💬 Response: {message}")
                except:
                    print("💬 Response received (could not parse message)")
            else:
                print("📥 Empty response received")
        except socket.timeout:
            print("⏰ No response received (timeout)")
        
        sock.close()
        print("✅ Connection closed")
        
    except Exception as e:
        print(f"❌ Error: {e}")

def main():
    """Interactive test client"""
    print("🧪 USSD Test Client")
    print("==================")
    print("This client sends USSD requests to test the forwarding system.")
    print("Make sure the server and Java client are running first.")
    print()
    
    while True:
        print("\nTest options:")
        print("1. Send *555# (should be forwarded to Java client)")
        print("2. Send *123# (built-in server response)")
        print("3. Send custom USSD code")
        print("4. Send follow-up (1, 2, 0, 3)")
        print("5. Exit")
        
        choice = input("\nEnter choice (1-5): ").strip()
        
        if choice == '1':
            send_ussd_request("*555#")
        elif choice == '2':
            send_ussd_request("*123#")
        elif choice == '3':
            ussd_code = input("Enter USSD code: ").strip()
            if ussd_code:
                send_ussd_request(ussd_code)
        elif choice == '4':
            follow_up = input("Enter follow-up (1, 2, 0, 3): ").strip()
            if follow_up:
                send_ussd_request(follow_up)
        elif choice == '5':
            print("👋 Goodbye!")
            break
        else:
            print("❌ Invalid choice")

if __name__ == "__main__":
    main()
