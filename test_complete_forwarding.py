#!/usr/bin/env python3
"""Complete end-to-end test of USSD forwarding to Java client with response handling"""

import socket
import struct
import time
import threading
from concurrent.futures import ThreadPoolExecutor

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
    """Enhanced Java client simulator that handles forwarded requests"""
    try:
        print("🟢 [JavaClient] Starting Java SMPP Client Simulator")
        
        # Connect to SMPP server
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.connect(('127.0.0.1', 2775))
        print("🟢 [JavaClient] Connected and binding as JavaClient...")
        
        # Bind as JavaClient
        bind_pdu = create_bind_transceiver("JavaClient", "forward123")
        sock.send(bind_pdu)
        
        # Receive bind response
        bind_response = sock.recv(1024)
        header = parse_pdu_header(bind_response)
        if header and header['command_status'] == 0:
            print("🟢 [JavaClient] Bind successful - ready to receive forwarded requests")
        else:
            print(f"🔴 [JavaClient] Bind failed")
            return
        
        sequence_counter = 2
        
        while True:
            try:
                # Read PDU
                data = sock.recv(4096)
                if not data:
                    break
                
                header = parse_pdu_header(data)
                if not header:
                    continue
                
                if header['command_id'] == 0x00000004:  # SUBMIT_SM (forwarded request)
                    parsed = parse_submit_sm(data)
                    if parsed:
                        msisdn = parsed['msisdn']
                        ussd_code = parsed['message']
                        
                        print(f"🟢 [JavaClient] 🎯 FORWARDED REQUEST: MSISDN={msisdn}, Code={ussd_code}")
                        
                        # Send SUBMIT_SM_RESP
                        submit_resp = create_submit_sm_resp(header['sequence_number'])
                        sock.send(submit_resp)
                        
                        # Generate response based on the USSD code
                        if ussd_code == "*555#":
                            response = "🏦 Java Banking Service\\n1. Account Info\\n2. Transfer\\n3. Exit"
                        elif ussd_code == "1":
                            response = "💰 Account: $1,500.00\\nAvailable: $1,450.00\\n\\n0. Back"
                        elif ussd_code == "2":
                            response = "💸 Transfer Service\\nEnter amount: \\n\\n0. Back"
                        elif ussd_code == "0":
                            response = "🏦 Java Banking Service\\n1. Account Info\\n2. Transfer\\n3. Exit"
                        elif ussd_code == "3":
                            response = "Thank you for using Java Banking! 👋"
                        else:
                            response = f"❓ Unknown command: {ussd_code}\\nPlease try again."
                        
                        # Send DELIVER_SM with response
                        deliver_sm = create_deliver_sm(msisdn, response, sequence_counter)
                        sequence_counter += 1
                        sock.send(deliver_sm)
                        print(f"🟢 [JavaClient] 📤 Sent response: {response[:30]}...")
                        
                elif header['command_id'] == 0x80000005:  # DELIVER_SM_RESP
                    print("🟢 [JavaClient] 📥 Received DELIVER_SM_RESP")
                    
                elif header['command_id'] == 0x00000015:  # ENQUIRE_LINK
                    # Send ENQUIRE_LINK_RESP
                    enquire_resp = struct.pack('>IIII', 16, 0x80000015, 0, header['sequence_number'])
                    sock.send(enquire_resp)
                    
            except Exception as e:
                print(f"🔴 [JavaClient] Error: {e}")
                break
        
        sock.close()
        print("🟢 [JavaClient] Connection closed")
        
    except Exception as e:
        print(f"🔴 [JavaClient] Client error: {e}")

def test_client():
    """Test client that sends USSD codes"""
    time.sleep(2)  # Wait for Java client to bind
    
    try:
        print("🔵 [TestClient] Starting test client...")
        
        # Connect to SMPP server
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.connect(('127.0.0.1', 2775))
        print("🔵 [TestClient] Connected to SMPP server")
        
        # Bind as test client
        bind_pdu = create_bind_transceiver("TestClient", "testpass")
        sock.send(bind_pdu)
        
        # Receive bind response
        bind_response = sock.recv(1024)
        header = parse_pdu_header(bind_response)
        if header and header['command_status'] == 0:
            print("🔵 [TestClient] Bind successful")
        else:
            print(f"🔴 [TestClient] Bind failed")
            return
        
        # Test sequence
        test_codes = ["*555#", "1", "2", "0", "3"]
        
        for i, code in enumerate(test_codes):
            print(f"\\n🔵 [TestClient] Testing USSD code: {code}")
            
            # Send USSD request
            submit_pdu = create_submit_sm("1234567890", code, i + 2)
            sock.send(submit_pdu)
            print(f"🔵 [TestClient] 📤 Sent SUBMIT_SM for {code}")
            
            # Wait for response
            timeout_count = 0
            while timeout_count < 5:
                try:
                    sock.settimeout(1.0)
                    data = sock.recv(4096)
                    if not data:
                        break
                    
                    header = parse_pdu_header(data)
                    if not header:
                        continue
                    
                    if header['command_id'] == 0x80000004:  # SUBMIT_SM_RESP
                        print("🔵 [TestClient] 📥 Received SUBMIT_SM_RESP")
                        
                    elif header['command_id'] == 0x00000005:  # DELIVER_SM (response)
                        print("🔵 [TestClient] 📥 Received DELIVER_SM (response from Java client)")
                        
                        # Send DELIVER_SM_RESP
                        deliver_resp = struct.pack('>IIII', 16, 0x80000005, 0, header['sequence_number'])
                        sock.send(deliver_resp)
                        break
                        
                    elif header['command_id'] == 0x00000015:  # ENQUIRE_LINK
                        enquire_resp = struct.pack('>IIII', 16, 0x80000015, 0, header['sequence_number'])
                        sock.send(enquire_resp)
                        
                    timeout_count = 0
                    
                except socket.timeout:
                    timeout_count += 1
                    continue
            
            time.sleep(1)  # Brief pause between tests
        
        print("\\n🔵 [TestClient] All tests completed!")
        sock.close()
        
    except Exception as e:
        print(f"🔴 [TestClient] Error: {e}")

def main():
    """Run the complete forwarding test"""
    print("🚀 Starting Complete USSD Forwarding Test")
    print("=" * 50)
    
    # Run Java client and test client in parallel
    with ThreadPoolExecutor(max_workers=2) as executor:
        java_future = executor.submit(java_client_simulator)
        test_future = executor.submit(test_client)
        
        # Wait for both to complete
        java_future.result()
        test_future.result()
    
    print("=" * 50)
    print("🎉 Complete forwarding test finished!")

if __name__ == "__main__":
    main()
