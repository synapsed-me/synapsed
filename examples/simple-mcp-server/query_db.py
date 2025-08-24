#!/usr/bin/env python3
"""Query and display the synapsed intents database"""

import sqlite3
import json
from datetime import datetime

def display_table(cursor, table_name, query):
    print(f"\n{'='*80}")
    print(f"QUERY: {query}")
    print(f"{'='*80}")
    
    cursor.execute(query)
    rows = cursor.fetchall()
    
    if not rows:
        print(f"No records found in {table_name}")
        return
    
    # Get column names
    columns = [description[0] for description in cursor.description]
    
    # Calculate column widths
    widths = [len(col) for col in columns]
    for row in rows:
        for i, val in enumerate(row):
            widths[i] = max(widths[i], len(str(val) if val is not None else "NULL"))
    
    # Print header
    header = " | ".join(col.ljust(widths[i]) for i, col in enumerate(columns))
    print(header)
    print("-" * len(header))
    
    # Print rows
    for row in rows:
        formatted_row = []
        for i, val in enumerate(row):
            if val is None:
                formatted_val = "NULL"
            elif isinstance(val, str) and (val.startswith('{') or val.startswith('[')):
                # Truncate JSON data for display
                formatted_val = val[:widths[i]-3] + "..." if len(val) > widths[i] else val
            else:
                formatted_val = str(val)
            formatted_row.append(formatted_val.ljust(widths[i]))
        print(" | ".join(formatted_row))
    
    print(f"\nTotal records: {len(rows)}")

def main():
    conn = sqlite3.connect("/tmp/synapsed_intents.db")
    cursor = conn.cursor()
    
    print("\n" + "="*80)
    print("SYNAPSED INTENTS DATABASE QUERY RESULTS")
    print("="*80)
    
    # Query intents table
    display_table(cursor, "intents", "SELECT * FROM intents")
    
    # Query agents table
    display_table(cursor, "agents", "SELECT * FROM agents")
    
    # Query verifications table with formatted evidence
    print(f"\n{'='*80}")
    print("QUERY: SELECT * FROM verifications")
    print(f"{'='*80}")
    
    cursor.execute("SELECT * FROM verifications")
    verifications = cursor.fetchall()
    
    for i, row in enumerate(verifications):
        print(f"\nVerification {i+1}:")
        print(f"  ID: {row[0]}")
        print(f"  Intent ID: {row[1]}")
        print(f"  Agent ID: {row[2]}")
        print(f"  Timestamp: {row[4]}")
        
        # Parse and display evidence
        evidence = json.loads(row[3])
        print(f"  Evidence:")
        for key, value in evidence.items():
            print(f"    - {key}: {value}")
    
    print(f"\nTotal verifications: {len(verifications)}")
    
    conn.close()
    print("\n" + "="*80)

if __name__ == "__main__":
    main()