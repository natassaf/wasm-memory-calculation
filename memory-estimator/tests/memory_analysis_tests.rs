use memory_estimator::*;
use std::fs;

#[test]
fn test_binary_size_categorization() {
    // Test tiny files
    assert_eq!(categorize_binary_size(25_000), "Tiny (< 50KB)");
    
    // Test small files
    assert_eq!(categorize_binary_size(75_000), "Small (50-100KB)");
    
    // Test medium files
    assert_eq!(categorize_binary_size(150_000), "Medium (100-200KB)");
    
    // Test large files
    assert_eq!(categorize_binary_size(300_000), "Large (200-500KB)");
    
    // Test very large files
    assert_eq!(categorize_binary_size(750_000), "Very Large (500KB-1MB)");
    
    // Test huge files
    assert_eq!(categorize_binary_size(2_000_000), "Huge (> 1MB)");
}

#[test]
fn test_ml_likelihood_estimation() {
    // Test very low likelihood
    assert_eq!(estimate_ml_likelihood(50_000), "Very Low (likely standard workload)");
    
    // Test low likelihood
    assert_eq!(estimate_ml_likelihood(150_000), "Low (possibly ML, but small model)");
    
    // Test medium likelihood
    assert_eq!(estimate_ml_likelihood(250_000), "Medium (likely ML with moderate model)");
    
    // Test high likelihood
    assert_eq!(estimate_ml_likelihood(400_000), "High (probably ML with large model)");
    
    // Test very high likelihood
    assert_eq!(estimate_ml_likelihood(1_000_000), "Very High (almost certainly ML with very large model)");
}

#[test]
fn test_matrix_component_analysis() {
    let wasm_file = "wasm_tasks/matrix_multiplication_component.cwasm";
    
    // Test binary size analysis
    let binary_info = analyze_binary_size(wasm_file).expect("Failed to analyze binary size");
    
    // Matrix component should be around 195KB
    assert!(binary_info.binary_size_bytes > 190_000);
    assert!(binary_info.binary_size_bytes < 200_000);
    
    // Should be categorized as medium
    assert_eq!(categorize_binary_size(binary_info.binary_size_bytes), "Medium (100-200KB)");
    
    // Should have low ML likelihood
    assert_eq!(estimate_ml_likelihood(binary_info.binary_size_bytes), "Low (possibly ML, but small model)");
}

#[test]
fn test_ml_component_analysis() {
    let wasm_file = "wasm_tasks/inference_component_onnx.cwasm";
    
    // Test binary size analysis
    let binary_info = analyze_binary_size(wasm_file).expect("Failed to analyze binary size");
    
    // ML component should be around 344KB
    assert!(binary_info.binary_size_bytes > 340_000);
    assert!(binary_info.binary_size_bytes < 350_000);
    
    // Should be categorized as large
    assert_eq!(categorize_binary_size(binary_info.binary_size_bytes), "Large (200-500KB)");
    
    // Should have high ML likelihood
    assert_eq!(estimate_ml_likelihood(binary_info.binary_size_bytes), "High (probably ML with large model)");
}

#[test]
fn test_wat_memory_analysis() {
    let wat_file = "wasm_tasks/matrix_multiplication_component.wat";
    
    // Only run this test if the WAT file exists
    if fs::metadata(wat_file).is_ok() {
        let memory_info = analyze_wat_memory_simple(wat_file).expect("Failed to analyze WAT memory");
        
        // Should have some memory pages
        assert!(memory_info.linear_memory_pages > 0);
        
        // Should have function tables
        assert!(!memory_info.function_tables.is_empty());
        
        // Should have reasonable memory estimates
        assert!(memory_info.estimated_minimum_memory > 0);
        assert!(memory_info.estimated_peak_memory > memory_info.estimated_minimum_memory);
    }
}

#[test]
fn test_memory_calculations() {
    // Test memory page calculations
    let pages = 17;
    let expected_bytes = pages as u64 * 65536; // 64KB per page
    assert_eq!(expected_bytes, 1_114_112); // 1.06 MB
    
    // Test buffer calculations
    let base_memory = 1_114_112; // 1.06 MB
    let stack_memory = 1_048_576; // 1 MB
    let buffer = 1_048_576; // 1 MB
    
    let total_memory = base_memory + stack_memory + buffer;
    assert_eq!(total_memory, 3_211_264); // ~3.06 MB
}

#[test]
fn test_file_existence() {
    // Test that our test files exist
    assert!(fs::metadata("wasm_tasks/matrix_multiplication_component.cwasm").is_ok());
    assert!(fs::metadata("wasm_tasks/inference_component_onnx.cwasm").is_ok());
}
