use memory_estimator::memory_info_estimator::detect_ml_task_from_wat;

fn main() {
    // Test with a known ML WAT file
    let ml_wat_file = "wasm-modules/image_classification_squeezenet_onnx_batch.wat";
    
    println!("Testing ML detection with: {}", ml_wat_file);
    
    match detect_ml_task_from_wat(ml_wat_file) {
        Ok(is_ml) => {
            println!("✅ Result: {}", if is_ml { "ML Task Detected" } else { "Not ML Task" });
        },
        Err(e) => {
            println!("❌ Error: {}", e);
        }
    }
    
    // Test with a known non-ML WAT file
    let non_ml_wat_file = "wasm-modules/fibonacci.wat";
    
    println!("\nTesting ML detection with: {}", non_ml_wat_file);
    
    match detect_ml_task_from_wat(non_ml_wat_file) {
        Ok(is_ml) => {
            println!("✅ Result: {}", if is_ml { "ML Task Detected" } else { "Not ML Task" });
        },
        Err(e) => {
            println!("❌ Error: {}", e);
        }
    }
}

