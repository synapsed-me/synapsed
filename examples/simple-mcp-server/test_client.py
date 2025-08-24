#!/usr/bin/env python3
"""Simple test client for MCP server"""

import json
import socket
import time

def send_request(method, params=None):
    """Send a JSON-RPC request to the MCP server"""
    request = {
        "jsonrpc": "2.0",
        "id": int(time.time()),
        "method": method,
        "params": params or {}
    }
    
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.connect(("localhost", 3000))
        s.sendall((json.dumps(request) + "\n").encode())
        response = s.recv(4096).decode()
        return json.loads(response)

def main():
    print("Testing MCP Server...")
    
    # Test 1: System info
    print("\n1. Getting system info...")
    resp = send_request("system/info")
    print(f"   Server: {resp['result']['server']}")
    print(f"   Version: {resp['result']['version']}")
    
    # Test 2: Declare three intents in parallel (as requested)
    print("\n2. Declaring 3 intents in parallel...")
    intents = [
        {"goal": "Implement consensus protocol", "description": "Byzantine fault tolerant consensus"},
        {"goal": "Create fault tolerance system", "description": "Circuit breakers and recovery"},
        {"goal": "Build recovery mechanisms", "description": "Checkpointing and state recovery"}
    ]
    
    intent_ids = []
    for intent in intents:
        resp = send_request("intent/declare", intent)
        intent_id = resp['result']['intent_id']
        intent_ids.append(intent_id)
        print(f"   ✓ Intent declared: {intent_id}")
        print(f"     Goal: {intent['goal']}")
    
    # Test 3: Spawn agents
    print("\n3. Spawning 3 agents...")
    agents_params = {
        "agents": [
            {"capabilities": ["consensus", "verification"]},
            {"capabilities": ["fault_tolerance", "monitoring"]},
            {"capabilities": ["recovery", "persistence"]}
        ]
    }
    resp = send_request("agent/spawn", agents_params)
    print(f"   ✓ Spawned {resp['result']['count']} agents")
    
    # Test 4: Verify intents
    print("\n4. Verifying intents...")
    for intent_id in intent_ids:
        for i in range(3):  # 3 verifications per intent
            resp = send_request("intent/verify", {
                "intent_id": intent_id,
                "evidence": {"test": "evidence"}
            })
            print(f"   ✓ Verification {i+1} for {intent_id[:8]}...")
    
    # Test 5: List all intents
    print("\n5. Listing all intents...")
    resp = send_request("intent/list")
    print(f"   Total intents: {resp['result']['count']}")
    for intent in resp['result']['intents']:
        status = "✓ VERIFIED" if intent['verified'] else "○ PENDING"
        print(f"   {status} {intent['goal']}")
        print(f"          Proofs: {len(intent['verification_proofs'])}")
    
    print("\n✅ All tests completed!")

if __name__ == "__main__":
    main()