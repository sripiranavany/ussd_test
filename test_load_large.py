#!/usr/bin/env python3
"""Test response percentage functionality for load testing"""

import socket
import struct
import binascii
import time
import threading
import statistics

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

def create_submit_sm(source_addr, destination_addr, short_message, sequence_number):
    """Create a submit_sm PDU"""
    command_id = 0x00000004  # SUBMIT_SM
    command_status = 0x00000000
    
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

def test_single_request(msisdn, request_id):
    """Test a single USSD request and measure response"""
    results = {
        'request_id': request_id,
        'msisdn': msisdn,
        'start_time': time.time(),
        'response_type': 'unknown',
        'response_time': None,
        'error_code': None,
        'success': False
    }
    
    try:
        # Connect to SMPP server
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.settimeout(10)  # 10 second timeout
        sock.connect(('127.0.0.1', 2775))
        
        # Bind
        bind_pdu = create_bind_transceiver("LoadTestClient", "testpass123")
        sock.send(bind_pdu)
        
        # Receive bind response
        bind_response = sock.recv(1024)
        
        # Send USSD request
        submit_pdu = create_submit_sm(msisdn, "123", "*123#", 2)
        sock.send(submit_pdu)
        
        # Try to receive submit response
        try:
            submit_response = sock.recv(1024)
            results['response_time'] = time.time() - results['start_time']
            
            # Parse response
            if len(submit_response) >= 16:
                header = struct.unpack('>IIII', submit_response[:16])
                command_status = header[2]
                
                if command_status == 0:
                    results['response_type'] = 'success'
                    results['success'] = True
                    
                    # Try to receive DELIVER_SM
                    try:
                        deliver_sm = sock.recv(1024)
                        # Send DELIVER_SM_RESP
                        deliver_resp_header = struct.pack('>IIII', 16, 0x80000005, 0, 2)
                        sock.send(deliver_resp_header)
                    except socket.timeout:
                        pass  # No DELIVER_SM received
                else:
                    results['response_type'] = 'failure'
                    results['error_code'] = command_status
                    results['success'] = False
        
        except socket.timeout:
            results['response_type'] = 'no_response'
            results['response_time'] = time.time() - results['start_time']
        
        sock.close()
        
    except Exception as e:
        results['response_type'] = 'error'
        results['error'] = str(e)
        results['response_time'] = time.time() - results['start_time']
    
    return results

def run_load_test(num_requests=100, num_threads=10):
    """Run load test with multiple concurrent requests"""
    print(f"Running load test with {num_requests} requests using {num_threads} threads...")
    
    results = []
    threads = []
    
    def worker(start_idx, end_idx):
        for i in range(start_idx, end_idx):
            msisdn = f"123456789{i:03d}"
            result = test_single_request(msisdn, i)
            results.append(result)
            print(f"Request {i}: {result['response_type']} ({result['response_time']:.3f}s)")
    
    # Split requests among threads
    requests_per_thread = num_requests // num_threads
    
    for t in range(num_threads):
        start_idx = t * requests_per_thread
        end_idx = start_idx + requests_per_thread
        if t == num_threads - 1:  # Last thread handles remaining requests
            end_idx = num_requests
        
        thread = threading.Thread(target=worker, args=(start_idx, end_idx))
        threads.append(thread)
        thread.start()
    
    # Wait for all threads to complete
    for thread in threads:
        thread.join()
    
    # Analyze results
    success_count = sum(1 for r in results if r['response_type'] == 'success')
    failure_count = sum(1 for r in results if r['response_type'] == 'failure')
    no_response_count = sum(1 for r in results if r['response_type'] == 'no_response')
    error_count = sum(1 for r in results if r['response_type'] == 'error')
    
    response_times = [r['response_time'] for r in results if r['response_time'] is not None]
    
    print(f"\n=== LOAD TEST RESULTS ===")
    print(f"Total requests: {num_requests}")
    print(f"Success responses: {success_count} ({success_count/num_requests*100:.1f}%)")
    print(f"Failure responses: {failure_count} ({failure_count/num_requests*100:.1f}%)")
    print(f"No responses: {no_response_count} ({no_response_count/num_requests*100:.1f}%)")
    print(f"Errors: {error_count} ({error_count/num_requests*100:.1f}%)")
    
    if response_times:
        print(f"\nResponse Time Statistics:")
        print(f"Average: {statistics.mean(response_times):.3f}s")
        print(f"Median: {statistics.median(response_times):.3f}s")
        print(f"Min: {min(response_times):.3f}s")
        print(f"Max: {max(response_times):.3f}s")
    
    return results

if __name__ == "__main__":
    print("SMPP Load Testing Tool")
    print("Testing response percentage configuration...")
    
    # Run small load test
    results = run_load_test(num_requests=200, num_threads=10)
    
    print("\nLoad test completed!")
