#!/usr/bin/env python3
"""
Visualize synapsed intent verification system results
Shows intents, agents, and verifications in a monitor-style format
"""

import sqlite3
import json
import os
from datetime import datetime
from collections import defaultdict

def format_timeline_event(timestamp, event_type, subject, data):
    """Format a timeline event for display"""
    time = datetime.fromisoformat(timestamp).strftime("%H:%M:%S.%f")[:-3]
    icon = {
        "intent.declared": "📋",
        "agent.spawned": "🤖",
        "intent.verified": "✅"
    }.get(event_type, "📌")
    
    return f"{time} {icon} {event_type:20} {subject[:8]}..."

def display_dashboard():
    """Display a monitoring dashboard view of the intent system"""
    
    print("\n" + "="*80)
    print(" " * 25 + "SYNAPSED INTENT MONITOR")
    print("="*80)
    
    # Connect to database
    conn = sqlite3.connect("/tmp/synapsed_intents.db")
    cursor = conn.cursor()
    
    # System Health Overview
    print("\n📊 SYSTEM HEALTH")
    print("-" * 40)
    
    cursor.execute("SELECT COUNT(*) FROM intents")
    intent_count = cursor.fetchone()[0]
    
    cursor.execute("SELECT COUNT(*) FROM intents WHERE verified = 1")
    verified_count = cursor.fetchone()[0]
    
    cursor.execute("SELECT COUNT(*) FROM agents")
    agent_count = cursor.fetchone()[0]
    
    cursor.execute("SELECT COUNT(*) FROM verifications")
    verification_count = cursor.fetchone()[0]
    
    print(f"  Intents:        {intent_count} total, {verified_count} verified")
    print(f"  Agents:         {agent_count} active")
    print(f"  Verifications:  {verification_count} completed")
    print(f"  Success Rate:   {(verified_count/intent_count*100 if intent_count > 0 else 0):.1f}%")
    
    # Current Intent Details
    print("\n🎯 CURRENT INTENT")
    print("-" * 40)
    
    cursor.execute("""
        SELECT id, goal, description, status, created_at, verification_count 
        FROM intents 
        ORDER BY created_at DESC 
        LIMIT 1
    """)
    intent = cursor.fetchone()
    
    if intent:
        print(f"  ID:     {intent[0]}")
        print(f"  Goal:   {intent[1]}")
        print(f"  Desc:   {intent[2]}")
        print(f"  Status: {intent[3].upper()} ({intent[5]} verifications)")
        print(f"  Time:   {intent[4]}")
    
    # Active Agents
    print("\n🤖 ACTIVE AGENTS")
    print("-" * 40)
    
    cursor.execute("""
        SELECT name, capabilities, trust_score 
        FROM agents 
        ORDER BY created_at DESC
    """)
    agents = cursor.fetchall()
    
    for agent in agents:
        caps = json.loads(agent[1])
        trust_bar = "█" * int(agent[2] * 10) + "░" * (10 - int(agent[2] * 10))
        print(f"  • {agent[0]:24} Trust: [{trust_bar}] {agent[2]:.1f}")
        print(f"    Capabilities: {', '.join(caps)}")
    
    # Verification Evidence
    print("\n🔍 VERIFICATION EVIDENCE")
    print("-" * 40)
    
    cursor.execute("""
        SELECT a.name, v.evidence, v.timestamp 
        FROM verifications v 
        JOIN agents a ON v.agent_id = a.id 
        ORDER BY v.timestamp DESC
    """)
    verifications = cursor.fetchall()
    
    for agent_name, evidence_json, timestamp in verifications:
        evidence = json.loads(evidence_json)
        time = datetime.fromisoformat(timestamp).strftime("%H:%M:%S")
        print(f"\n  [{time}] {agent_name}:")
        
        for key, value in evidence.items():
            if isinstance(value, bool):
                icon = "✅" if value else "❌"
                print(f"    • {key}: {icon}")
            else:
                print(f"    • {key}: {value}")
    
    # Event Timeline from Substrates log
    print("\n📈 EVENT TIMELINE (from Substrates)")
    print("-" * 40)
    
    if os.path.exists("/tmp/synapsed_substrates.log"):
        with open("/tmp/synapsed_substrates.log", "r") as f:
            lines = f.readlines()
            
        events = []
        for line in lines:
            if line.strip() and line.startswith("{"):
                try:
                    event = json.loads(line)
                    events.append(event)
                except:
                    pass
        
        # Show last session's events
        session_start = None
        for i, line in enumerate(lines):
            if "NEW DEMONSTRATION SESSION" in line:
                session_start = i
        
        if session_start:
            session_events = []
            for line in lines[session_start:]:
                if line.strip() and line.startswith("{"):
                    try:
                        event = json.loads(line)
                        session_events.append(event)
                    except:
                        pass
            
            for event in session_events:
                formatted = format_timeline_event(
                    event["timestamp"],
                    event["event_type"],
                    event["subject"],
                    event.get("data", {})
                )
                print(f"  {formatted}")
    
    # Performance Metrics
    print("\n⚡ PERFORMANCE METRICS")
    print("-" * 40)
    
    # Calculate average verification time
    cursor.execute("""
        SELECT 
            MIN(timestamp) as first_verify,
            MAX(timestamp) as last_verify,
            COUNT(*) as verify_count
        FROM verifications
        WHERE intent_id = ?
    """, (intent[0] if intent else "",))
    
    perf = cursor.fetchone()
    if perf and perf[0] and perf[1]:
        first = datetime.fromisoformat(perf[0])
        last = datetime.fromisoformat(perf[1])
        duration = (last - first).total_seconds()
        print(f"  Total Verification Time:  {duration:.3f}s")
        print(f"  Parallel Verifications:   {perf[2]}")
        print(f"  Avg Time per Agent:       {duration/perf[2] if perf[2] > 0 else 0:.3f}s")
    
    conn.close()
    
    print("\n" + "="*80)
    print(" " * 20 + "MONITORING DASHBOARD COMPLETE")
    print("="*80 + "\n")

if __name__ == "__main__":
    display_dashboard()