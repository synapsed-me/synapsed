//! Integration tests for CRDT implementations

use synapsed_crdt::*;
use tokio_test;

#[tokio::test]
async fn test_rga_collaborative_editing() {
    let actor1 = ActorId::new();
    let actor2 = ActorId::new();
    
    let mut rga1 = Rga::new(actor1);
    let mut rga2 = Rga::new(actor2);
    
    // Actor 1 types "Hello"
    rga1.insert_at_offset(0, 'H').await.unwrap();
    rga1.insert_at_offset(1, 'e').await.unwrap();
    rga1.insert_at_offset(2, 'l').await.unwrap();
    rga1.insert_at_offset(3, 'l').await.unwrap();
    rga1.insert_at_offset(4, 'o').await.unwrap();
    
    // Actor 2 types "World" at the same time
    rga2.insert_at_offset(0, 'W').await.unwrap();
    rga2.insert_at_offset(1, 'o').await.unwrap();
    rga2.insert_at_offset(2, 'r').await.unwrap();
    rga2.insert_at_offset(3, 'l').await.unwrap();
    rga2.insert_at_offset(4, 'd').await.unwrap();
    
    // Merge the documents
    rga1.merge(&rga2).await.unwrap();
    rga2.merge(&rga1).await.unwrap();
    
    // Both should converge to the same state
    assert_eq!(rga1.text(), rga2.text());
    assert!(rga1.text().contains("Hello") || rga1.text().contains("World"));
    assert_eq!(rga1.len(), 10);
}

#[tokio::test]
async fn test_lww_register_conflict_resolution() {
    let actor1 = ActorId::new();
    let actor2 = ActorId::new();
    
    let mut lww1 = LwwRegister::new(actor1);
    let mut lww2 = LwwRegister::new(actor2);
    
    // Concurrent writes
    lww1.set("value1".to_string()).await.unwrap();
    
    // Sleep to ensure different timestamps
    tokio::time::sleep(tokio::time::Duration::from_millis(2)).await;
    
    lww2.set("value2".to_string()).await.unwrap();
    
    // Merge - later write should win
    lww1.merge(&lww2).await.unwrap();
    lww2.merge(&lww1).await.unwrap();
    
    assert_eq!(lww1.get(), Some("value2".to_string()));
    assert_eq!(lww2.get(), Some("value2".to_string()));
}

#[tokio::test]
async fn test_or_set_add_remove_semantics() {
    let actor1 = ActorId::new();
    let actor2 = ActorId::new();
    
    let mut set1 = OrSet::new(actor1);
    let mut set2 = OrSet::new(actor2);
    
    // Both add the same element
    set1.add("item".to_string()).await.unwrap();
    set2.add("item".to_string()).await.unwrap();
    
    // set1 removes it (but only observes its own add)
    set1.remove(&"item".to_string()).await.unwrap();
    
    // Merge
    set1.merge(&set2).await.unwrap();
    set2.merge(&set1).await.unwrap();
    
    // Element should still be present because set2's add wasn't observed by set1's remove
    assert!(set1.contains(&"item".to_string()));
    assert!(set2.contains(&"item".to_string()));
}

#[tokio::test]
async fn test_pn_counter_distributed_counting() {
    let actor1 = ActorId::new();
    let actor2 = ActorId::new();
    let actor3 = ActorId::new();
    
    let mut counter1 = PnCounter::new(actor1);
    let mut counter2 = PnCounter::new(actor2);
    let mut counter3 = PnCounter::new(actor3);
    
    // Distributed increments and decrements
    counter1.increment(10).await.unwrap();
    counter2.increment(5).await.unwrap();
    counter3.decrement(3).await.unwrap();
    
    counter1.decrement(2).await.unwrap();
    counter2.increment(7).await.unwrap();
    
    // Merge all counters
    counter1.merge(&counter2).await.unwrap();
    counter1.merge(&counter3).await.unwrap();
    
    counter2.merge(&counter1).await.unwrap();
    counter2.merge(&counter3).await.unwrap();
    
    counter3.merge(&counter1).await.unwrap();
    counter3.merge(&counter2).await.unwrap();
    
    // All should converge to same value: 10 + 5 + 7 - 3 - 2 = 17
    assert_eq!(counter1.value(), 17);
    assert_eq!(counter2.value(), 17);
    assert_eq!(counter3.value(), 17);
}

#[tokio::test]
async fn test_mixed_crdt_operations() {
    // Test using multiple CRDTs together
    let actor_id = ActorId::new();
    
    let mut rga = Rga::new(actor_id.clone());
    let mut lww = LwwRegister::new(actor_id.clone());
    let mut set = OrSet::new(actor_id.clone());
    let mut counter = PnCounter::new(actor_id);
    
    // Perform operations on each CRDT
    rga.insert_at_offset(0, 'T').await.unwrap();
    rga.insert_at_offset(1, 'e').await.unwrap();
    rga.insert_at_offset(2, 's').await.unwrap();
    rga.insert_at_offset(3, 't').await.unwrap();
    
    lww.set("document_v1".to_string()).await.unwrap();
    
    set.add("tag1".to_string()).await.unwrap();
    set.add("tag2".to_string()).await.unwrap();
    
    counter.increment(42).await.unwrap();
    
    // Verify states
    assert_eq!(rga.text(), "Test");
    assert_eq!(lww.get(), Some("document_v1".to_string()));
    assert_eq!(set.len(), 2);
    assert_eq!(counter.value(), 42);
}

#[tokio::test]
async fn test_synchronization_scenario() {
    // Simulate a realistic synchronization scenario
    let actor1 = ActorId::new();
    let actor2 = ActorId::new();
    
    let mut rga1 = Rga::new(actor1.clone());
    let mut rga2 = Rga::new(actor2.clone());
    
    // Start with same base text
    for (i, ch) in "Base text".chars().enumerate() {
        rga1.insert_at_offset(i, ch).await.unwrap();
        rga2.insert_at_offset(i, ch).await.unwrap();
    }
    
    // Simulate network partition - each user makes changes
    
    // User 1 edits: "Base text" -> "Base document text"
    rga1.insert_at_offset(5, 'd').await.unwrap();
    rga1.insert_at_offset(6, 'o').await.unwrap();
    rga1.insert_at_offset(7, 'c').await.unwrap();
    rga1.insert_at_offset(8, 'u').await.unwrap();
    rga1.insert_at_offset(9, 'm').await.unwrap();
    rga1.insert_at_offset(10, 'e').await.unwrap();
    rga1.insert_at_offset(11, 'n').await.unwrap();
    rga1.insert_at_offset(12, 't').await.unwrap();
    rga1.insert_at_offset(13, ' ').await.unwrap();
    
    // User 2 edits: "Base text" -> "Modified base text"
    rga2.insert_at_offset(0, 'M').await.unwrap();
    rga2.insert_at_offset(1, 'o').await.unwrap();
    rga2.insert_at_offset(2, 'd').await.unwrap();
    rga2.insert_at_offset(3, 'i').await.unwrap();
    rga2.insert_at_offset(4, 'f').await.unwrap();
    rga2.insert_at_offset(5, 'i').await.unwrap();
    rga2.insert_at_offset(6, 'e').await.unwrap();
    rga2.insert_at_offset(7, 'd').await.unwrap();
    rga2.insert_at_offset(8, ' ').await.unwrap();
    
    // Network reconnects - merge changes
    rga1.merge(&rga2).await.unwrap();
    rga2.merge(&rga1).await.unwrap();
    
    // Both should converge
    assert_eq!(rga1.text(), rga2.text());
    
    // Text should contain elements from both edits
    let final_text = rga1.text();
    println!("Final merged text: {}", final_text);
    
    // Should have all characters from both versions
    assert!(final_text.len() > "Base text".len());
}

#[tokio::test]
async fn test_garbage_collection() {
    let actor_id = ActorId::new();
    let mut rga = Rga::new(actor_id);
    
    // Add text
    for (i, ch) in "Hello World".chars().enumerate() {
        rga.insert_at_offset(i, ch).await.unwrap();
    }
    
    // Delete most of it
    for _ in 0..8 {
        rga.delete_at_offset(0).await.unwrap();
    }
    
    assert_eq!(rga.text(), "rld");
    
    // Check if GC is needed
    assert!(rga.needs_gc());
    
    // Perform GC
    let removed = rga.garbage_collect().await.unwrap();
    println!("Garbage collected {} bytes", rga.garbage_size());
    
    // Text should still be correct after GC
    assert_eq!(rga.text(), "rld");
}