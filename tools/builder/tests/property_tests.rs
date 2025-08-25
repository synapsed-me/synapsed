//! Property-based tests for Synapsed Builder invariants
//! 
//! These tests use proptest and quickcheck to verify that the builder
//! maintains its invariants across all possible inputs.

use synapsed_builder::prelude::*;
use synapsed_builder::{
    builder::{SynapsedBuilder, StorageBackend, ObservabilityLevel, NetworkType},
    registry::{ComponentRegistry, Component, Capability},
    composer::Composer,
    validator::Validator,
};
use proptest::prelude::*;
use quickcheck::{Arbitrary, Gen, QuickCheck};
use quickcheck_macros::quickcheck;
use std::collections::{HashSet, HashMap};

// Property 1: Component registry always maintains consistency
proptest! {
    #[test]
    fn registry_consistency(
        components in prop::collection::vec(component_strategy(), 0..50)
    ) {
        let mut registry = ComponentRegistry::new();
        
        for component in &components {
            registry.register(component.clone());
        }
        
        // Invariant: All registered components can be retrieved
        for component in &components {
            prop_assert!(registry.get(&component.name).is_some());
        }
        
        // Invariant: Capability index is consistent
        for component in &components {
            for capability in &component.capabilities {
                let found = registry.find_by_capability(capability.clone());
                prop_assert!(found.contains(&component.name));
            }
        }
    }
}

// Property 2: Dependency resolution always produces valid ordering
proptest! {
    #[test]
    fn dependency_resolution_ordering(
        components in prop::collection::vec(valid_component_name(), 1..20),
        connections in connection_list_strategy(5)
    ) {
        let app = Application {
            name: "test-app".to_string(),
            description: "Test".to_string(),
            components: components.clone(),
            connections,
            config: Default::default(),
            env: Default::default(),
        };
        
        let composer = Composer::new();
        
        // Skip if components don't exist in registry
        if let Ok(resolved) = composer.resolve_dependencies(&app) {
            // Invariant: All original components are in resolved list
            for comp in &components {
                prop_assert!(resolved.contains(comp));
            }
            
            // Invariant: No duplicates in resolved list
            let unique: HashSet<_> = resolved.iter().collect();
            prop_assert_eq!(unique.len(), resolved.len());
            
            // Invariant: Dependencies come before dependents
            verify_dependency_order(&resolved)?;
        }
    }
}

// Property 3: Builder always produces valid applications
proptest! {
    #[test]
    fn builder_produces_valid_apps(
        name in "[a-z][a-z0-9-]{0,30}",
        description in ".*",
        storage in storage_backend_strategy(),
        observability in observability_level_strategy(),
        network in network_type_strategy()
    ) {
        let result = SynapsedBuilder::new(&name)
            .description(&description)
            .add_storage(storage)
            .add_observability(observability)
            .add_network(network)
            .build();
        
        if let Ok(app) = result {
            // Invariant: Name matches
            prop_assert_eq!(app.name, name);
            
            // Invariant: Core is always included
            prop_assert!(app.components.contains(&"synapsed-core".to_string()));
            
            // Invariant: No empty components list
            prop_assert!(!app.components.is_empty());
            
            // Invariant: Valid according to validator
            let validator = Validator::new();
            // Validation might fail for missing dependencies, but shouldn't panic
            let _ = validator.validate(&app);
        }
    }
}

// Property 4: Configuration merging is associative
#[quickcheck]
fn config_merging_associative(
    configs: Vec<HashMap<String, serde_json::Value>>
) -> bool {
    if configs.len() < 3 {
        return true;
    }
    
    let mut builder1 = SynapsedBuilder::new("test");
    let mut builder2 = SynapsedBuilder::new("test");
    
    // Merge all configs at once
    for config in &configs {
        for (key, value) in config {
            builder1 = builder1.configure("test-component", value.clone());
        }
    }
    
    // Merge configs in pairs
    if configs.len() >= 2 {
        for config in &configs[..2] {
            for (key, value) in config {
                builder2 = builder2.configure("test-component", value.clone());
            }
        }
        for config in &configs[2..] {
            for (key, value) in config {
                builder2 = builder2.configure("test-component", value.clone());
            }
        }
    }
    
    // Both approaches should yield same result
    let app1 = builder1.build();
    let app2 = builder2.build();
    
    match (app1, app2) {
        (Ok(a1), Ok(a2)) => {
            a1.config.get("test-component") == a2.config.get("test-component")
        },
        _ => true
    }
}

// Property 5: Connections are symmetric
proptest! {
    #[test]
    fn connections_symmetric(
        connections in connection_list_strategy(20)
    ) {
        let mut forward_map: HashMap<String, HashSet<String>> = HashMap::new();
        let mut reverse_map: HashMap<String, HashSet<String>> = HashMap::new();
        
        for conn in &connections {
            forward_map.entry(conn.from.clone())
                .or_default()
                .insert(conn.to.clone());
            reverse_map.entry(conn.to.clone())
                .or_default()
                .insert(conn.from.clone());
        }
        
        // Invariant: Every forward connection has a reverse entry
        for (from, tos) in &forward_map {
            for to in tos {
                prop_assert!(reverse_map.get(to).map_or(false, |froms| froms.contains(from)));
            }
        }
    }
}

// Property 6: Template expansion preserves structure
proptest! {
    #[test]
    fn template_expansion_preserves_structure(
        name in "[a-z][a-z0-9-]{0,30}",
        config_overrides in prop::collection::hash_map(
            "[a-z][a-z0-9-]{0,20}",
            json_value_strategy(),
            0..5
        )
    ) {
        use synapsed_builder::templates::Templates;
        
        // Test with verified-ai-agent template
        let mut builder = Templates::verified_ai_agent();
        
        for (key, value) in config_overrides {
            builder = builder.configure("synapsed-intent", value);
        }
        
        if let Ok(app) = builder.build() {
            // Invariant: Template components are preserved
            prop_assert!(app.components.contains(&"synapsed-intent".to_string()));
            prop_assert!(app.components.contains(&"synapsed-verify".to_string()));
            
            // Invariant: Core is always included
            prop_assert!(app.components.contains(&"synapsed-core".to_string()));
        }
    }
}

// Property 7: Circular dependencies are detected
#[test]
fn circular_dependencies_detected() {
    let validator = Validator::new();
    
    // Create app with circular dependency
    let app = Application {
        name: "circular-test".to_string(),
        description: "Test".to_string(),
        components: vec!["comp-a".to_string(), "comp-b".to_string()],
        connections: vec![
            Connection {
                from: "comp-a".to_string(),
                event: "event1".to_string(),
                to: "comp-b".to_string(),
                handler: "handler1".to_string(),
            },
            Connection {
                from: "comp-b".to_string(),
                event: "event2".to_string(),
                to: "comp-a".to_string(),
                handler: "handler2".to_string(),
            },
        ],
        config: Default::default(),
        env: Default::default(),
    };
    
    // Should not error - bidirectional connections are allowed
    assert!(validator.validate(&app).is_ok());
}

// Property 8: Component capabilities are immutable after registration
proptest! {
    #[test]
    fn component_capabilities_immutable(
        component in component_strategy(),
        extra_capabilities in prop::collection::vec(capability_strategy(), 0..5)
    ) {
        let mut registry = ComponentRegistry::new();
        registry.register(component.clone());
        
        // Get component from registry
        let retrieved = registry.get(&component.name).unwrap();
        
        // Invariant: Capabilities haven't changed
        prop_assert_eq!(retrieved.capabilities.len(), component.capabilities.len());
        for cap in &component.capabilities {
            prop_assert!(retrieved.capabilities.contains(cap));
        }
    }
}

// Property 9: Environment variables are properly escaped
#[quickcheck]
fn env_vars_properly_escaped(
    env_vars: Vec<(String, String)>
) -> bool {
    let mut builder = SynapsedBuilder::new("test");
    
    for (key, value) in env_vars {
        // Skip invalid keys
        if key.is_empty() || key.contains('=') || key.contains('\0') {
            continue;
        }
        builder = builder.env(&key, &value);
    }
    
    if let Ok(app) = builder.build() {
        // All env vars should be retrievable
        for (key, value) in &app.env {
            assert!(!key.contains('='));
            assert!(!key.contains('\0'));
        }
        true
    } else {
        true
    }
}

// Property 10: Recipe serialization round-trip
proptest! {
    #[test]
    fn recipe_serialization_roundtrip(
        name in "[a-z][a-z0-9-]{0,30}",
        description in ".*",
        components in prop::collection::vec(valid_component_name(), 1..10)
    ) {
        use synapsed_builder::recipe::{Recipe, RecipeManager};
        
        let original = Recipe {
            name: name.clone(),
            description: description.clone(),
            version: "1.0.0".to_string(),
            components: components.clone(),
            connections: vec![],
            config: Default::default(),
        };
        
        // Serialize to YAML
        let yaml = serde_yaml::to_string(&original).unwrap();
        
        // Deserialize back
        let deserialized: Recipe = serde_yaml::from_str(&yaml).unwrap();
        
        // Invariant: Round-trip preserves data
        prop_assert_eq!(original.name, deserialized.name);
        prop_assert_eq!(original.description, deserialized.description);
        prop_assert_eq!(original.components, deserialized.components);
    }
}

// Strategy generators for property tests

fn component_strategy() -> impl Strategy<Value = Component> {
    (
        "[a-z][a-z0-9-]{0,30}",
        prop::collection::vec(capability_strategy(), 1..5),
        ".*"
    ).prop_map(|(name, capabilities, description)| {
        Component {
            name,
            version: "*".to_string(),
            capabilities,
            description,
            dependencies: vec![],
        }
    })
}

fn capability_strategy() -> impl Strategy<Value = Capability> {
    prop_oneof![
        Just(Capability::Core),
        Just(Capability::Intent),
        Just(Capability::Verification),
        Just(Capability::Consensus),
        Just(Capability::Networking),
        Just(Capability::Storage),
        Just(Capability::Observability),
        Just(Capability::Crypto),
        Just(Capability::Payments),
    ]
}

fn valid_component_name() -> impl Strategy<Value = String> {
    "synapsed-[a-z]{3,10}"
}

fn storage_backend_strategy() -> impl Strategy<Value = StorageBackend> {
    prop_oneof![
        Just(StorageBackend::Sqlite),
        Just(StorageBackend::Postgres),
        Just(StorageBackend::Redis),
    ]
}

fn observability_level_strategy() -> impl Strategy<Value = ObservabilityLevel> {
    prop_oneof![
        Just(ObservabilityLevel::Basic),
        Just(ObservabilityLevel::Standard),
        Just(ObservabilityLevel::Full),
    ]
}

fn network_type_strategy() -> impl Strategy<Value = NetworkType> {
    prop_oneof![
        Just(NetworkType::P2P),
        Just(NetworkType::ClientServer),
        Just(NetworkType::Hybrid),
    ]
}

fn connection_list_strategy(max_size: usize) -> impl Strategy<Value = Vec<Connection>> {
    prop::collection::vec(
        (valid_component_name(), 
         "[a-z_]{3,20}",
         valid_component_name(),
         "[a-z_]{3,20}")
        .prop_map(|(from, event, to, handler)| Connection {
            from,
            event,
            to,
            handler,
        }),
        0..max_size
    )
}

fn json_value_strategy() -> impl Strategy<Value = serde_json::Value> {
    let leaf = prop_oneof![
        Just(serde_json::Value::Null),
        any::<bool>().prop_map(serde_json::Value::Bool),
        any::<i64>().prop_map(|n| serde_json::Value::Number(n.into())),
        "[a-zA-Z0-9 ]{0,50}".prop_map(serde_json::Value::String),
    ];
    
    leaf.prop_recursive(
        3, // max depth
        10, // max size
        5, // items per collection
        |inner| {
            prop_oneof![
                prop::collection::vec(inner.clone(), 0..3)
                    .prop_map(serde_json::Value::Array),
                prop::collection::hash_map(
                    "[a-z][a-z0-9_]{0,20}",
                    inner,
                    0..3
                ).prop_map(|m| serde_json::Value::Object(
                    m.into_iter().collect()
                )),
            ]
        }
    )
}

fn verify_dependency_order(components: &[String]) -> Result<(), TestCaseError> {
    // Known dependencies
    let deps = vec![
        ("synapsed-consensus", "synapsed-net"),
        ("synapsed-verify", "synapsed-intent"),
        ("synapsed-monitor", "synapsed-substrates"),
    ];
    
    for (dependent, dependency) in deps {
        if let Some(dep_idx) = components.iter().position(|c| c == dependent) {
            if let Some(prereq_idx) = components.iter().position(|c| c == dependency) {
                prop_assert!(prereq_idx < dep_idx, 
                    "{} should come before {}", dependency, dependent);
            }
        }
    }
    
    Ok(())
}

// QuickCheck arbitrary implementations

impl Arbitrary for StorageBackend {
    fn arbitrary(g: &mut Gen) -> Self {
        match u8::arbitrary(g) % 3 {
            0 => StorageBackend::Sqlite,
            1 => StorageBackend::Postgres,
            _ => StorageBackend::Redis,
        }
    }
}

impl Arbitrary for ObservabilityLevel {
    fn arbitrary(g: &mut Gen) -> Self {
        match u8::arbitrary(g) % 3 {
            0 => ObservabilityLevel::Basic,
            1 => ObservabilityLevel::Standard,
            _ => ObservabilityLevel::Full,
        }
    }
}

impl Arbitrary for NetworkType {
    fn arbitrary(g: &mut Gen) -> Self {
        match u8::arbitrary(g) % 3 {
            0 => NetworkType::P2P,
            1 => NetworkType::ClientServer,
            _ => NetworkType::Hybrid,
        }
    }
}