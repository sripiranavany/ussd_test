#!/usr/bin/env python3
"""Run a larger load test for more accurate percentage measurements"""

import subprocess
import time

def run_larger_test():
    """Run a larger test with more requests for better accuracy"""
    print("Running larger load test (200 requests) for accurate percentage measurement...")
    
    # Create a modified test script for larger load
    with open('test_load_percentage.py', 'r') as f:
        content = f.read()
    
    # Modify the test to run 200 requests
    modified_content = content.replace(
        'results = run_load_test(num_requests=50, num_threads=5)',
        'results = run_load_test(num_requests=200, num_threads=10)'
    )
    
    with open('test_load_large.py', 'w') as f:
        f.write(modified_content)
    
    # Run the test
    result = subprocess.run(['python3', 'test_load_large.py'], 
                          capture_output=True, text=True)
    
    print(result.stdout)
    if result.stderr:
        print("STDERR:", result.stderr)

if __name__ == "__main__":
    run_larger_test()
