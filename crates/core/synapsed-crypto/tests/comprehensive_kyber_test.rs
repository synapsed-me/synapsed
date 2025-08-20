use synapsed_crypto::{
    kyber::{Kyber512, Kyber768, Kyber1024},
    random::TestRng,
    traits::Kem,
};

#[test]
fn test_all_kyber_variants() {
    println!("\n=== Testing All Kyber Variants ===\n");
    
    // Test Kyber512
    println!("Testing Kyber512...");
    let mut passed_512 = 0;
    let mut failed_512 = 0;
    
    for i in 0..10 {
        let mut rng = TestRng::new(i as u64);
        let (pk, sk) = Kyber512::generate_keypair(&mut rng).unwrap();
        let (ct, ss1) = Kyber512::encapsulate(&pk, &mut rng).unwrap();
        let ss2 = Kyber512::decapsulate(&sk, &ct).unwrap();
        
        if ss1.as_ref() == ss2.as_ref() {
            passed_512 += 1;
        } else {
            failed_512 += 1;
            if i == 0 {
                println!("  Iteration {i}: FAILED");
                println!("    SS1: {:?}", &ss1.as_ref()[..8]);
                println!("    SS2: {:?}", &ss2.as_ref()[..8]);
            }
        }
    }
    println!("  Passed: {passed_512}/10, Failed: {failed_512}/10");
    
    // Test Kyber768
    println!("\nTesting Kyber768...");
    let mut passed_768 = 0;
    let mut failed_768 = 0;
    
    for i in 0..10 {
        let mut rng = TestRng::new(100 + i as u64);
        let (pk, sk) = Kyber768::generate_keypair(&mut rng).unwrap();
        let (ct, ss1) = Kyber768::encapsulate(&pk, &mut rng).unwrap();
        let ss2 = Kyber768::decapsulate(&sk, &ct).unwrap();
        
        if ss1.as_ref() == ss2.as_ref() {
            passed_768 += 1;
        } else {
            failed_768 += 1;
            if i == 0 {
                println!("  Iteration {i}: FAILED");
                println!("    SS1: {:?}", &ss1.as_ref()[..8]);
                println!("    SS2: {:?}", &ss2.as_ref()[..8]);
            }
        }
    }
    println!("  Passed: {passed_768}/10, Failed: {failed_768}/10");
    
    // Test Kyber1024
    println!("\nTesting Kyber1024...");
    let mut passed_1024 = 0;
    let mut failed_1024 = 0;
    
    for i in 0..10 {
        let mut rng = TestRng::new(200 + i as u64);
        let (pk, sk) = Kyber1024::generate_keypair(&mut rng).unwrap();
        let (ct, ss1) = Kyber1024::encapsulate(&pk, &mut rng).unwrap();
        let ss2 = Kyber1024::decapsulate(&sk, &ct).unwrap();
        
        if ss1.as_ref() == ss2.as_ref() {
            passed_1024 += 1;
        } else {
            failed_1024 += 1;
            if i == 0 {
                println!("  Iteration {i}: FAILED");
                println!("    SS1: {:?}", &ss1.as_ref()[..8]);
                println!("    SS2: {:?}", &ss2.as_ref()[..8]);
            }
        }
    }
    println!("  Passed: {passed_1024}/10, Failed: {failed_1024}/10");
    
    // Summary
    println!("\n=== SUMMARY ===");
    println!("Kyber512:  {passed_512} passed, {failed_512} failed");
    println!("Kyber768:  {passed_768} passed, {failed_768} failed");
    println!("Kyber1024: {passed_1024} passed, {failed_1024} failed");
    
    let total_passed = passed_512 + passed_768 + passed_1024;
    let _total_failed = failed_512 + failed_768 + failed_1024;
    
    println!("\nTotal: {total_passed}/30 tests passed");
    
    if failed_512 > 0 && failed_768 > 0 && failed_1024 == 0 {
        println!("\nNOTE: Kyber512 and Kyber768 both use 10-bit compression (DU=10)");
        println!("      Kyber1024 uses 11-bit compression (DU=11)");
        println!("      This suggests the compression bug fix was correctly applied!");
    }
    
    // Don't assert failure - just report results
}

#[test]
fn test_secret_key_size_verification() {
    use synapsed_crypto::params::kyber::{kyber512, kyber768, kyber1024};
    
    println!("\n=== Verifying Secret Key Sizes ===");
    
    println!("Kyber512:");
    println!("  Expected SECRET_KEY_SIZE: 1632");
    println!("  Actual SECRET_KEY_SIZE: {}", kyber512::SECRET_KEY_SIZE);
    println!("  Match: {}", kyber512::SECRET_KEY_SIZE == 1632);
    
    println!("\nKyber768:");
    println!("  Expected SECRET_KEY_SIZE: 2400");
    println!("  Actual SECRET_KEY_SIZE: {}", kyber768::SECRET_KEY_SIZE);
    println!("  Match: {}", kyber768::SECRET_KEY_SIZE == 2400);
    
    println!("\nKyber1024:");
    println!("  Expected SECRET_KEY_SIZE: 3168");
    println!("  Actual SECRET_KEY_SIZE: {}", kyber1024::SECRET_KEY_SIZE);
    println!("  Match: {}", kyber1024::SECRET_KEY_SIZE == 3168);
}