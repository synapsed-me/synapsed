//! Demo of the trust persistence system
//!
//! This example demonstrates how to use the persistent storage system
//! for trust scores with different storage backends.

use chrono::Utc;
use std::path::Path;
use tempfile::TempDir;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

use synapsed_swarm::{
    persistence::{InMemoryTrustStore, FileTrustStore, TrustStore, StorageHealth},
    trust::{TrustManager, TrustScore, TrustUpdateReason, BackupConfig},
    types::AgentId,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("🚀 Synapsed Trust Persistence Demo");
    println!("=====================================");

    // Create test agents
    let agent1 = AgentId::new_v4();
    let agent2 = AgentId::new_v4();
    let agent3 = AgentId::new_v4();

    println!("📝 Created test agents:");
    println!("  Agent 1: {}", agent1);
    println!("  Agent 2: {}", agent2);
    println!("  Agent 3: {}", agent3);

    // Demo 1: In-Memory Storage
    println!("\n🧠 Demo 1: In-Memory Trust Storage");
    println!("----------------------------------");
    demo_in_memory_storage(agent1, agent2).await?;

    // Demo 2: File-Based Storage
    println!("\n📁 Demo 2: File-Based Trust Storage");
    println!("-----------------------------------");
    demo_file_storage(agent1, agent2, agent3).await?;

    // Demo 3: Trust Manager with Persistent Storage
    println!("\n⚙️  Demo 3: Trust Manager with Persistence");
    println!("------------------------------------------");
    demo_trust_manager_persistence(agent1, agent2).await?;

    // Demo 4: Backup and Restore
    println!("\n💾 Demo 4: Backup and Restore");
    println!("-----------------------------");
    demo_backup_restore(agent1, agent2).await?;

    println!("\n✅ All demos completed successfully!");
    Ok(())
}

async fn demo_in_memory_storage(
    agent1: AgentId,
    agent2: AgentId,
) -> Result<(), Box<dyn std::error::Error>> {
    let store = InMemoryTrustStore::new();
    store.initialize().await?;

    println!("📊 Storing initial trust scores...");
    
    // Store some trust scores
    let score1 = TrustScore::new(0.8);
    let score2 = TrustScore::new(0.6);
    
    store.store_trust_score(agent1, score1).await?;
    store.store_trust_score(agent2, score2).await?;

    // Retrieve and display
    let retrieved1 = store.get_trust_score(agent1).await?;
    let retrieved2 = store.get_trust_score(agent2).await?;

    println!("  Agent 1 trust: {:?}", retrieved1);
    println!("  Agent 2 trust: {:?}", retrieved2);

    // Update a score
    let mut updated_score = score1;
    updated_score.update(true, true);
    store.store_trust_score(agent1, updated_score).await?;

    println!("  Agent 1 updated trust: {:.3}", updated_score.value);

    // Health check
    let health = store.health_check().await?;
    println!("📋 Storage health: {} agents, {} updates", 
             health.total_agents, health.total_updates);

    Ok(())
}

async fn demo_file_storage(
    agent1: AgentId,
    agent2: AgentId,
    agent3: AgentId,
) -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let store = FileTrustStore::new(temp_dir.path(), None)?;
    store.initialize().await?;

    println!("📊 Working with file-based storage...");
    
    // Store trust scores
    let scores = [
        (agent1, TrustScore::new(0.9)),
        (agent2, TrustScore::new(0.7)),
        (agent3, TrustScore::new(0.5)),
    ];

    for (agent_id, score) in &scores {
        store.store_trust_score(*agent_id, *score).await?;
        println!("  Stored trust for {}: {:.3}", agent_id, score.value);
    }

    // Retrieve all scores
    let all_scores = store.get_all_trust_scores().await?;
    println!("📊 Retrieved {} trust scores from storage", all_scores.len());

    // Test transactions
    println!("💳 Testing transaction support...");
    {
        let mut tx = store.begin_transaction().await?;
        let new_score = TrustScore::new(0.95);
        tx.store_trust_score(agent1, new_score).await?;
        tx.commit().await?;
        println!("  Transaction committed successfully");
    }

    let final_score = store.get_trust_score(agent1).await?;
    println!("  Final Agent 1 trust: {:?}", final_score);

    Ok(())
}

async fn demo_trust_manager_persistence(
    agent1: AgentId,
    agent2: AgentId,
) -> Result<(), Box<dyn std::error::Error>> {
    let store = std::sync::Arc::new(InMemoryTrustStore::new());
    
    // Create trust manager with custom backup config
    let backup_config = BackupConfig {
        enabled: true,
        interval_secs: 30, // Short interval for demo
        on_significant_change: true,
        significant_change_threshold: 0.1,
    };

    let trust_manager = TrustManager::with_storage(store)
        .with_backup_config(backup_config);

    trust_manager.initialize().await?;

    println!("🎯 Initializing agents with trust manager...");
    
    // Initialize agents
    trust_manager.initialize_agent(agent1, 0.7).await?;
    trust_manager.initialize_agent(agent2, 0.5).await?;

    // Simulate trust updates
    println!("📈 Simulating trust updates...");
    
    // Successful task execution
    trust_manager.update_trust(agent1, true, true).await?;
    println!("  Agent 1 completed verified task");

    // Failed task
    trust_manager.update_trust(agent2, false, false).await?;
    println!("  Agent 2 failed task");

    // Promise fulfillment
    trust_manager.update_trust_for_promise(agent1, true).await?;
    println!("  Agent 1 fulfilled promise");

    // Get current trust levels
    let trust1 = trust_manager.get_trust(agent1).await?;
    let trust2 = trust_manager.get_trust(agent2).await?;

    println!("📊 Current trust levels:");
    println!("  Agent 1: {:.3}", trust1);
    println!("  Agent 2: {:.3}", trust2);

    // Get trust history
    let history1 = trust_manager.get_history(agent1, Some(5)).await?;
    println!("📜 Agent 1 has {} trust updates in history", history1.len());

    // Get trusted agents
    let trusted = trust_manager.get_trusted_agents(0.6).await?;
    println!("🏆 Found {} trusted agents (threshold: 0.6)", trusted.len());

    // Apply time decay
    println!("⏰ Applying time decay...");
    trust_manager.apply_time_decay(0.01).await?;

    let health = trust_manager.get_storage_health().await?;
    println!("💚 Storage health check: {} agents tracked", health.total_agents);

    Ok(())
}

async fn demo_backup_restore(
    agent1: AgentId,
    agent2: AgentId,
) -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let store1 = std::sync::Arc::new(FileTrustStore::new(temp_dir.path(), None)?);
    
    let trust_manager1 = TrustManager::with_storage(store1);
    trust_manager1.initialize().await?;

    println!("💾 Creating initial trust data...");
    
    // Create some trust data
    trust_manager1.initialize_agent(agent1, 0.8).await?;
    trust_manager1.initialize_agent(agent2, 0.6).await?;
    
    trust_manager1.update_trust(agent1, true, true).await?;
    trust_manager1.update_trust(agent2, false, false).await?;

    let initial_trust1 = trust_manager1.get_trust(agent1).await?;
    let initial_trust2 = trust_manager1.get_trust(agent2).await?;

    println!("  Initial Agent 1 trust: {:.3}", initial_trust1);
    println!("  Initial Agent 2 trust: {:.3}", initial_trust2);

    // Create backup
    let backup_path = temp_dir.path().join("trust_backup");
    trust_manager1.create_backup(&backup_path).await?;
    println!("💾 Created backup at: {:?}", backup_path);

    // Create new trust manager and restore
    let temp_dir2 = TempDir::new()?;
    let store2 = std::sync::Arc::new(FileTrustStore::new(temp_dir2.path(), None)?);
    let trust_manager2 = TrustManager::with_storage(store2);
    trust_manager2.initialize().await?;

    println!("🔄 Restoring from backup...");
    trust_manager2.restore_backup(&backup_path).await?;

    // Verify restored data
    let restored_trust1 = trust_manager2.get_trust(agent1).await?;
    let restored_trust2 = trust_manager2.get_trust(agent2).await?;

    println!("  Restored Agent 1 trust: {:.3}", restored_trust1);
    println!("  Restored Agent 2 trust: {:.3}", restored_trust2);

    // Verify values match
    assert!((initial_trust1 - restored_trust1).abs() < 0.001);
    assert!((initial_trust2 - restored_trust2).abs() < 0.001);
    
    println!("✅ Backup and restore verified successfully!");

    Ok(())
}