//! Verify Kyber constants are correct

use synapsed_crypto::params::kyber::{kyber512, kyber768, kyber1024};

#[test]
fn test_kyber512_constants() {
    println!("Kyber512 constants:");
    println!("  PUBLIC_KEY_SIZE: {}", kyber512::PUBLIC_KEY_SIZE);
    println!("  SECRET_KEY_SIZE: {}", kyber512::SECRET_KEY_SIZE);
    println!("  POLYVECBYTES: {}", kyber512::POLYVECBYTES);
    
    // Verify the values are correct
    assert_eq!(kyber512::PUBLIC_KEY_SIZE, 800, "PUBLIC_KEY_SIZE should be 800");
    assert_eq!(kyber512::SECRET_KEY_SIZE, 1632, "SECRET_KEY_SIZE should be 1632, not 1664!");
}

#[test]
fn test_kyber768_constants() {
    println!("Kyber768 constants:");
    println!("  PUBLIC_KEY_SIZE: {}", kyber768::PUBLIC_KEY_SIZE);
    println!("  SECRET_KEY_SIZE: {}", kyber768::SECRET_KEY_SIZE);
    println!("  POLYVECBYTES: {}", kyber768::POLYVECBYTES);
    
    assert_eq!(kyber768::PUBLIC_KEY_SIZE, 1184);
    assert_eq!(kyber768::SECRET_KEY_SIZE, 2400);
}

#[test]
fn test_kyber1024_constants() {
    println!("Kyber1024 constants:");
    println!("  PUBLIC_KEY_SIZE: {}", kyber1024::PUBLIC_KEY_SIZE);
    println!("  SECRET_KEY_SIZE: {}", kyber1024::SECRET_KEY_SIZE);
    println!("  POLYVECBYTES: {}", kyber1024::POLYVECBYTES);
    
    assert_eq!(kyber1024::PUBLIC_KEY_SIZE, 1568);
    assert_eq!(kyber1024::SECRET_KEY_SIZE, 3168);
}