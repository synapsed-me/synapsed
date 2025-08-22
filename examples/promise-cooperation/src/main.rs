//! Promise Theory Cooperation Example
//! 
//! This example demonstrates how autonomous AI agents can cooperate using
//! Promise Theory principles, ensuring voluntary cooperation without coercion.

use anyhow::Result;
use synapsed_promise::{
    AutonomousAgent, Promise, PromiseState, Imposition,
    TrustModel, CooperationProtocol, PromiseOutcome,
};
use synapsed_intent::{HierarchicalIntent, IntentBuilder, StepAction};
use synapsed_substrates::{BasicCircuit, BasicChannel, Subject, Emission};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, debug, warn};
use uuid::Uuid;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("debug")
        .init();

    info!("Starting Promise Theory Cooperation Example");
    
    // Example 1: Basic promise exchange between agents
    basic_promise_exchange().await?;
    
    // Example 2: Trust model and reputation
    trust_model_example().await?;
    
    // Example 3: Cooperation protocol for task delegation
    cooperation_protocol().await?;
    
    // Example 4: Handling impositions (requests from other agents)
    imposition_handling().await?;
    
    // Example 5: Complex multi-agent scenario
    multi_agent_scenario().await?;
    
    info!("All examples completed successfully!");
    Ok(())
}

/// Example 1: Basic promise exchange
async fn basic_promise_exchange() -> Result<()> {
    info!("=== Example 1: Basic Promise Exchange ===");
    
    // Create two autonomous agents
    let alice = Arc::new(AutonomousAgent::new("alice", HashMap::new()));
    let bob = Arc::new(AutonomousAgent::new("bob", HashMap::new()));
    
    info!("Created agents: Alice and Bob");
    
    // Alice makes a promise to Bob
    let promise = Promise::new(
        "I will process your data within 5 seconds",
        alice.id().to_string(),
        bob.id().to_string(),
    );
    
    info!("Alice creates promise: '{}'", promise.body());
    
    // Bob must accept the promise (voluntary cooperation)
    let accepted_promise = promise.accept();
    info!("Bob accepts the promise");
    
    // Alice fulfills the promise
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    let fulfilled_promise = accepted_promise.fulfill();
    info!("Alice fulfills the promise");
    
    match fulfilled_promise.state() {
        PromiseState::Fulfilled => info!("✓ Promise successfully fulfilled"),
        _ => warn!("✗ Promise not fulfilled"),
    }
    
    Ok(())
}

/// Example 2: Trust model and reputation
async fn trust_model_example() -> Result<()> {
    info!("=== Example 2: Trust Model and Reputation ===");
    
    // Create a trust model
    let mut trust_model = TrustModel::new();
    
    // Add agents with initial trust scores
    trust_model.add_agent("agent_1", 0.5); // Neutral trust
    trust_model.add_agent("agent_2", 0.8); // High trust
    trust_model.add_agent("agent_3", 0.2); // Low trust
    
    info!("Initial trust scores:");
    info!("  Agent 1: {:.2}", trust_model.get_trust("agent_1"));
    info!("  Agent 2: {:.2}", trust_model.get_trust("agent_2"));
    info!("  Agent 3: {:.2}", trust_model.get_trust("agent_3"));
    
    // Simulate promise fulfillment/violation
    trust_model.update_trust("agent_1", true);  // Fulfilled promise
    trust_model.update_trust("agent_2", true);  // Fulfilled promise
    trust_model.update_trust("agent_3", false); // Violated promise
    
    info!("\nUpdated trust scores after promises:");
    info!("  Agent 1: {:.2} (↑ fulfilled)", trust_model.get_trust("agent_1"));
    info!("  Agent 2: {:.2} (↑ fulfilled)", trust_model.get_trust("agent_2"));
    info!("  Agent 3: {:.2} (↓ violated)", trust_model.get_trust("agent_3"));
    
    // Decision making based on trust
    let threshold = 0.6;
    for agent in ["agent_1", "agent_2", "agent_3"] {
        if trust_model.is_trustworthy(agent, threshold) {
            info!("✓ {} is trustworthy for delegation", agent);
        } else {
            warn!("✗ {} is not trustworthy enough", agent);
        }
    }
    
    Ok(())
}

/// Example 3: Cooperation protocol
async fn cooperation_protocol() -> Result<()> {
    info!("=== Example 3: Cooperation Protocol ===");
    
    // Create a cooperation protocol
    let protocol = CooperationProtocol::new("data_processing_protocol");
    
    // Define protocol rules
    let rules = vec![
        "Agents must declare capabilities before accepting tasks",
        "Processing time must not exceed promised duration",
        "Results must be verifiable",
        "Agents can refuse tasks beyond their capabilities",
    ];
    
    info!("Protocol '{}' rules:", protocol.name());
    for (i, rule) in rules.iter().enumerate() {
        info!("  {}. {}", i + 1, rule);
    }
    
    // Create agents that follow the protocol
    let processor = Arc::new(AutonomousAgent::new(
        "data_processor",
        HashMap::from([
            ("capability".to_string(), "data_processing".to_string()),
            ("max_size".to_string(), "1GB".to_string()),
        ])
    ));
    
    let validator = Arc::new(AutonomousAgent::new(
        "validator",
        HashMap::from([
            ("capability".to_string(), "validation".to_string()),
            ("algorithms".to_string(), "sha256,md5".to_string()),
        ])
    ));
    
    // Processor promises to handle data
    let processing_promise = Promise::new(
        "Process CSV data and return JSON",
        processor.id().to_string(),
        validator.id().to_string(),
    );
    
    // Validator promises to verify results
    let validation_promise = Promise::new(
        "Validate JSON output against schema",
        validator.id().to_string(),
        processor.id().to_string(),
    );
    
    info!("✓ Cooperation protocol established between agents");
    
    Ok(())
}

/// Example 4: Handling impositions
async fn imposition_handling() -> Result<()> {
    info!("=== Example 4: Handling Impositions ===");
    
    // Create an agent with specific capabilities
    let agent = Arc::new(AutonomousAgent::new(
        "specialized_agent",
        HashMap::from([
            ("expertise".to_string(), "image_processing".to_string()),
            ("max_resolution".to_string(), "4K".to_string()),
        ])
    ));
    
    // Create impositions (requests from other agents)
    let impositions = vec![
        Imposition::new(
            "Process this 1080p image",
            "requester_1",
            agent.id(),
        ),
        Imposition::new(
            "Process this 8K image", // Beyond capability
            "requester_2",
            agent.id(),
        ),
        Imposition::new(
            "Analyze this text document", // Wrong expertise
            "requester_3",
            agent.id(),
        ),
    ];
    
    info!("Agent '{}' evaluating {} impositions:", agent.id(), impositions.len());
    
    for imposition in impositions {
        let can_handle = evaluate_imposition(&agent, &imposition).await;
        
        if can_handle {
            info!("  ✓ Accepts: '{}'", imposition.request());
            
            // Convert to promise
            let promise = Promise::new(
                imposition.request(),
                agent.id().to_string(),
                imposition.requester().to_string(),
            );
            let _ = promise.accept();
        } else {
            info!("  ✗ Rejects: '{}' (beyond capabilities)", imposition.request());
        }
    }
    
    Ok(())
}

/// Example 5: Complex multi-agent scenario
async fn multi_agent_scenario() -> Result<()> {
    info!("=== Example 5: Multi-Agent Scenario ===");
    info!("Scenario: Distributed data pipeline with 4 specialized agents");
    
    // Create specialized agents
    let collector = Arc::new(AutonomousAgent::new("collector", HashMap::new()));
    let processor = Arc::new(AutonomousAgent::new("processor", HashMap::new()));
    let analyzer = Arc::new(AutonomousAgent::new("analyzer", HashMap::new()));
    let reporter = Arc::new(AutonomousAgent::new("reporter", HashMap::new()));
    
    // Create observability circuit
    let circuit = Arc::new(BasicCircuit::new("pipeline_circuit"));
    let subject = Subject::new("pipeline", "promises");
    let channel = Arc::new(BasicChannel::new(subject.clone()));
    circuit.add_channel(channel.clone());
    let pipe = channel.create_pipe("promise_events");
    
    // Build promise chain
    let promises = vec![
        Promise::new(
            "Collect data from APIs",
            collector.id().to_string(),
            processor.id().to_string(),
        ),
        Promise::new(
            "Transform and clean data",
            processor.id().to_string(),
            analyzer.id().to_string(),
        ),
        Promise::new(
            "Perform statistical analysis",
            analyzer.id().to_string(),
            reporter.id().to_string(),
        ),
        Promise::new(
            "Generate visualization report",
            reporter.id().to_string(),
            "user".to_string(),
        ),
    ];
    
    info!("Promise chain created:");
    for (i, promise) in promises.iter().enumerate() {
        info!("  {}. {} → {}: {}", 
            i + 1,
            promise.promiser(),
            promise.promisee(),
            promise.body()
        );
        
        // Emit promise creation event
        pipe.emit(Emission::new(
            format!("Promise created: {}", promise.id()),
            subject.clone()
        ));
    }
    
    // Simulate promise execution with trust updates
    let mut trust_model = TrustModel::new();
    trust_model.add_agent(collector.id(), 0.7);
    trust_model.add_agent(processor.id(), 0.7);
    trust_model.add_agent(analyzer.id(), 0.7);
    trust_model.add_agent(reporter.id(), 0.7);
    
    info!("\nExecuting promise chain:");
    for promise in promises {
        let promiser = promise.promiser();
        
        // Accept and fulfill with probability based on trust
        let accepted = promise.accept();
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        
        let success = trust_model.get_trust(promiser) > 0.5;
        if success {
            let _ = accepted.fulfill();
            trust_model.update_trust(promiser, true);
            info!("  ✓ {} fulfilled promise", promiser);
            
            pipe.emit(Emission::new(
                format!("Promise fulfilled: {}", promiser),
                subject.clone()
            ));
        } else {
            trust_model.update_trust(promiser, false);
            warn!("  ✗ {} violated promise", promiser);
            
            pipe.emit(Emission::new(
                format!("Promise violated: {}", promiser),
                subject.clone()
            ));
        }
    }
    
    info!("\nFinal trust scores:");
    for agent in [collector, processor, analyzer, reporter] {
        let trust = trust_model.get_trust(agent.id());
        info!("  {}: {:.2}", agent.id(), trust);
    }
    
    Ok(())
}

/// Helper function to evaluate if an agent can handle an imposition
async fn evaluate_imposition(agent: &Arc<AutonomousAgent>, imposition: &Imposition) -> bool {
    // Simple capability matching (in real scenario, would be more sophisticated)
    let request = imposition.request().to_lowercase();
    
    if let Some(expertise) = agent.capabilities().get("expertise") {
        if expertise == "image_processing" && request.contains("image") {
            // Check resolution limits
            if request.contains("8k") {
                return false; // Beyond capability
            }
            return true;
        }
    }
    
    false
}