#!/usr/bin/env python3
"""
Simple ACP client to test the goose ACP agent.
Connects to goose acp running on stdio.
"""

import subprocess
import json
import sys
import uuid

class AcpClient:
    def __init__(self):
        # Start the goose acp process
        self.process = subprocess.Popen(
            ['cargo', 'run', '-p', 'goose-cli', '--', 'acp'],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=0
        )
        self.request_id = 0
        
    def send_request(self, method, params=None):
        self.request_id += 1
        request = {
            "jsonrpc": "2.0",
            "method": method,
            "id": self.request_id,
        }
        if params:
            request["params"] = params
        
        # Send the request
        request_str = json.dumps(request)
        print(f">>> Sending: {request_str}")
        self.process.stdin.write(request_str + '\n')
        self.process.stdin.flush()
        
        # Read response
        response_line = self.process.stdout.readline()
        if not response_line:
            return None
            
        print(f"<<< Response: {response_line}")
        return json.loads(response_line)
    
    def initialize(self):
        return self.send_request("initialize", {
            "protocolVersion": "v1",
            "clientCapabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        })
    
    def new_session(self):
        return self.send_request("newSession", {
            "context": {}
        })
    
    def prompt(self, session_id, text):
        return self.send_request("prompt", {
            "sessionId": session_id,
            "prompt": [
                {
                    "type": "text",
                    "text": text
                }
            ]
        })
    
    def close(self):
        if self.process:
            self.process.terminate()
            self.process.wait()

def main():
    print("Starting ACP client test...")
    client = AcpClient()
    
    try:
        # Initialize the agent
        print("\n1. Initializing agent...")
        init_response = client.initialize()
        if init_response and 'result' in init_response:
            print(f"   Initialized successfully: {init_response['result']}")
        else:
            print(f"   Failed to initialize: {init_response}")
            return
        
        # Create a new session
        print("\n2. Creating new session...")
        session_response = client.new_session()
        if session_response and 'result' in session_response:
            session_id = session_response['result']['sessionId']
            print(f"   Created session: {session_id}")
        else:
            print(f"   Failed to create session: {session_response}")
            return
        
        # Send a prompt
        print("\n3. Sending prompt...")
        prompt_response = client.prompt(session_id, "Hello! What is 2 + 2?")
        if prompt_response:
            print(f"   Got response: {prompt_response}")
        else:
            print("   Failed to get prompt response")
            
    finally:
        client.close()
        print("\nTest complete.")

if __name__ == "__main__":
    main()
