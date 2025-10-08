use memory_estimator::memory_info_estimator::*;
use memory_estimator::memory_info_monitor::*;
use std::fs;
use base64::{Engine as _, engine::general_purpose};
use flate2::write::GzEncoder;
use flate2::Compression;
use std::io::Write;
use serde_json::json;

pub fn estimate_memory_info(wasm_file: &str, wat_file: &str) -> MemoryInfo {
    let memory_info = build_memory_info(wasm_file, wat_file);
    memory_info
}

pub async fn measure_memory_info(wasm_file: &str, wat_file: &str, payload: &str) -> MemoryInfo {
    println!("🔬 Measuring actual memory usage for: {}", wasm_file);

    // First, get the estimated memory info
    let mut estimated_info = build_memory_info(wasm_file, wat_file);

    // Try to run the actual WASM component and measure memory with monitoring
    let result = run_wasm_job_component_with_memory_monitoring(0, wasm_file.to_string(), "run".to_string(), payload.to_string(), "models/model_1".to_string()).await;

    match result {
        Ok((val, monitor)) => {
            println!("✅ WASM execution completed successfully");
            println!("📊 Execution result: {:?}", val);

            // Update the memory info with actual execution data
            estimated_info.is_ml_workload = true; // Mark as ML since it executed successfully
            
            // Update with actual measured memory usage
            estimated_info.estimated_minimum_memory_bytes = monitor.initial_memory_bytes;
            estimated_info.estimated_peak_memory_bytes = monitor.peak_memory_bytes;
            
            println!("📈 Memory measurement completed");
            estimated_info
        },
        Err(e) => {
            println!("❌ WASM execution failed: {:?}", e);
            println!("⚠️  Falling back to estimated memory info");
            estimated_info
        }
    }
}

#[tokio::main]
async fn main() {
    println!("WASM Memory Analyzer (Simplified)");
    // Example usage
    let wasm_file = "wasm_tasks/inference_component_onnx.wasm";
    let cwasm_file = "wasm_tasks/inference_component_onnx.cwasm";
    let wat_file = "wasm_tasks/inference_component_onnx.wat";
    
     // Convert WASM to WAT
     match convert_wasm_to_wat(cwasm_file, wat_file) {
        Ok(_) => println!("Successfully converted {} to {}", wasm_file, wat_file),
        Err(e) => println!("Error converting file: {}", e),
    }

    let memory_info = estimate_memory_info(cwasm_file, wat_file);

    println!("Memory info: {}", memory_info);
    // print_memory_analysis_simple(&memory_info);

    // Encode rhino.jpg into base64
    let image_data = fs::read("rhino.jpg").expect("Failed to read rhino.jpg");
    let image_base64 = general_purpose::STANDARD.encode(&image_data);
    
    // Create the JSON payload that the WASM component expects
    let json_payload = json!({
        "model_path": "models/model_1/squeezenet1.1-7.onnx",
        "labels_path": "models/model_1/squeezenet1.1-7.txt", 
        "input": image_base64
    });
    
    let payload_string = json_payload.to_string();
    println!("📦 JSON payload size: {} bytes", payload_string);
    
    // Send the JSON payload directly (no compression, no base64 encoding)
    let payload = payload_string;
    
    measure_memory_info(wasm_file, wat_file, &payload).await;

    println!("Memory info: {}", memory_info);
}