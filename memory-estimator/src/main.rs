use memory_estimator::*;

fn build_memory_info(wasm_file: &str, wat_file: &str) -> MemoryInfo {
    let mut memory_info = MemoryInfo::new();
    
    // Analyze binary size first (fast and informative)
    match analyze_binary_size(wasm_file, &mut memory_info) {
        Ok(_) => {
            let size_category = categorize_binary_size(memory_info.binary_size_bytes);
            println!("📦 Binary Analysis:");
            println!("   • File size: {:.2} MB", memory_info.binary_size_mb);
            println!("   • Size category: {}", size_category);
        },
        Err(e) => println!("Error analyzing binary: {}", e),
    }
    println!("memory_info: {}", memory_info);
    // Analyze memory requirements (simplified)
    match analyze_wat_memory_simple(wat_file, &mut memory_info) {
        Ok(()) => {},
        Err(e) => println!("Error analyzing memory: {}", e),
    }

    calculate_aggregated_memory(&mut memory_info);

    memory_info
}

fn main() {
    println!("WASM Memory Analyzer (Simplified)");
    // Example usage
    let wasm_file = "wasm_tasks/inference_component_onnx.cwasm";
    let wat_file = "wasm_tasks/inference_component_onnx.wat";
    
     // Convert WASM to WAT
     match convert_wasm_to_wat(wasm_file, wat_file) {
        Ok(_) => println!("Successfully converted {} to {}", wasm_file, wat_file),
        Err(e) => println!("Error converting file: {}", e),
    }

    let memory_info = build_memory_info(&wasm_file, &wat_file);

    println!("Memory info: {}", memory_info);
    print_memory_analysis_simple(&memory_info);
}