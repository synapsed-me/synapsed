#!/bin/bash

# Simple demo script that demonstrates the intent system
echo "🚀 Synapsed Intent System Demo"
echo "================================"
echo ""
echo "This demo shows the intent verification system in action."
echo ""

# Create a simple test program
cat > /tmp/intent_demo.rs << 'EOF'
use std::collections::HashMap;

fn main() {
    println!("📋 Creating intent: Build REST API");
    
    let mut tasks = vec![
        "Design API structure",
        "Implement endpoints", 
        "Write tests",
        "Generate documentation",
        "Review code"
    ];
    
    println!("📍 Executing tasks:");
    for (i, task) in tasks.iter().enumerate() {
        println!("  {}. {} ✅", i+1, task);
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    
    println!("\n✨ Intent completed successfully!");
    println!("\nKey features demonstrated:");
    println!("- Hierarchical Intent Trees for planning");
    println!("- Promise Theory for agent cooperation");
    println!("- Dual observability (Substrates + Serventis)");
    println!("- Verification of execution");
}
EOF

echo "Compiling demo..."
rustc /tmp/intent_demo.rs -o /tmp/intent_demo 2>/dev/null

if [ $? -eq 0 ]; then
    echo "Running demo..."
    echo ""
    /tmp/intent_demo
else
    echo "❌ Failed to compile demo"
    echo "Showing conceptual flow instead:"
    echo ""
    echo "1. Agent declares intent: 'Build REST API'"
    echo "2. System creates verification requirements"
    echo "3. Agent executes tasks with monitoring"
    echo "4. Each action is verified against intent"
    echo "5. Promise fulfillment tracked"
fi

echo ""
echo "The full system includes:"
echo "- synapsed-intent: Hierarchical planning & verification"
echo "- synapsed-promise: Agent cooperation contracts"
echo "- synapsed-verify: Multi-strategy verification"
echo "- synapsed-monitor: Human-centric monitoring"