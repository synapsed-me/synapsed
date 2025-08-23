use synapsed_intent::{intent::IntentBuilder, types::ContextBounds};

let intent = IntentBuilder::new("Fix all Axum Handler trait errors in monitor REST API")
    .with_precondition("11 Handler trait errors exist")
    .with_postcondition("All Handler errors resolved")
    .with_postcondition("cargo build --bin monitor-server shows 0 Handler errors")
    .step("Analyze Handler trait requirements", "Research why handlers fail")
    .step("Fix handler signatures", "Update all 11 handler functions")
    .step("Verify compilation", "Run cargo build and verify success")
    .build();

println!("Intent declared: {:?}", intent);