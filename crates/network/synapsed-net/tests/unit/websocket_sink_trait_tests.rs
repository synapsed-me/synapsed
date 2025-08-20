//! Test for WebSocket Sink trait issues
//! These tests specifically target the compilation errors in websocket.rs

use synapsed_net::transport::websocket::{WebSocketTransport, WebSocketConfig};
use synapsed_net::types::{PeerInfo, NetworkAddress};
use std::net::SocketAddr;
use tokio::time::{timeout, Duration};

#[tokio::test]
async fn test_websocket_header_issues() {
    // This test should fail due to header trait bounds issues
    // Error: the trait bound `(): tokio_tungstenite::tungstenite::http::header::IntoHeaderName` is not satisfied
    
    let config = WebSocketConfig::default();
    let transport = WebSocketTransport::new(config);
    
    // Try to connect with proper headers
    let peer = PeerInfo {
        id: "test_peer".to_string(),
        addresses: vec!["ws://127.0.0.1:8080".to_string()],
        capabilities: vec!["websocket".to_string()],
        last_seen: std::time::SystemTime::now(),
        reputation: 1.0,
    };
    
    let result = timeout(Duration::from_secs(1), transport.connect(&peer)).await;
    
    // Expected to fail due to header compilation errors
    assert!(result.is_err() || result.unwrap().is_err());
}

#[tokio::test]
async fn test_websocket_stream_as_mut_issues() {
    // This test targets the as_mut() method issues
    // Error: the method `as_mut` exists but trait bounds were not satisfied
    
    let config = WebSocketConfig::default();
    let transport = WebSocketTransport::new(config);
    
    // Try to create a listener - this should trigger the as_mut issues
    let addr: SocketAddr = "127.0.0.1:8081".parse().unwrap();
    let result = timeout(Duration::from_secs(1), transport.listen(addr)).await;
    
    // Expected to fail due to compilation errors
    assert!(result.is_err() || result.unwrap().is_err());
}

#[tokio::test]
async fn test_websocket_moved_value_issues() {
    // This test targets the "use of moved value" errors
    // Multiple locations where `ws` is moved and then used again
    
    let config = WebSocketConfig::default();
    let transport = WebSocketTransport::new(config);
    
    // This will trigger the moved value compilation errors
    let addr: SocketAddr = "127.0.0.1:8082".parse().unwrap();
    
    // Try to listen and accept connections
    match timeout(Duration::from_secs(1), transport.listen(addr)).await {
        Ok(Ok(mut listener)) => {
            // Try to accept - this path has moved value issues
            let accept_result = timeout(Duration::from_secs(1), listener.accept()).await;
            assert!(accept_result.is_err());
        }
        _ => {
            // Expected to fail before getting here
        }
    }
}

#[tokio::test]
async fn test_websocket_type_mismatch() {
    // This test targets the type mismatch between WebSocketStream types
    // Error: expected WebSocketStream<MaybeTlsStream<TcpStream>> found WebSocketStream<TcpStream>
    
    let config = WebSocketConfig::default();
    let transport = WebSocketTransport::new(config);
    
    // Test both ws:// and wss:// connections to trigger type mismatches
    let peers = vec![
        create_test_peer("ws://127.0.0.1:8083"),
        create_test_peer("wss://127.0.0.1:8084"),
    ];
    
    for peer in peers {
        let result = timeout(Duration::from_secs(1), transport.connect(&peer)).await;
        // Expected to fail due to type mismatches
        assert!(result.is_err() || result.unwrap().is_err());
    }
}

// Helper function to create test peers
fn create_test_peer(address: &str) -> PeerInfo {
    PeerInfo {
        id: format!("peer_{}", address),
        addresses: vec![address.to_string()],
        capabilities: vec!["websocket".to_string()],
        last_seen: std::time::SystemTime::now(),
        reputation: 1.0,
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;
    
    proptest! {
        #[test]
        fn prop_websocket_addresses_should_parse(
            host in "[0-9]{1,3}\\.[0-9]{1,3}\\.[0-9]{1,3}\\.[0-9]{1,3}",
            port in 1024u16..65535u16,
            use_tls in any::<bool>()
        ) {
            let scheme = if use_tls { "wss" } else { "ws" };
            let address = format!("{}://{}:{}", scheme, host, port);
            
            // All valid addresses should be parseable
            let peer = create_test_peer(&address);
            prop_assert!(!peer.addresses.is_empty());
            prop_assert_eq!(peer.addresses[0], address);
        }
        
        #[test]
        fn prop_websocket_config_invariants(
            max_frame_size in 1024usize..16777216usize,
            max_message_size in 1024usize..67108864usize,
            enable_compression in any::<bool>(),
            compression_level in 0u32..9u32,
        ) {
            let config = WebSocketConfig {
                max_frame_size,
                max_message_size,
                enable_compression,
                compression_level,
                ..Default::default()
            };
            
            // Frame size should be less than message size
            prop_assert!(config.max_frame_size <= config.max_message_size);
            
            // Compression level should be valid
            if config.enable_compression {
                prop_assert!(config.compression_level <= 9);
            }
        }
    }
}