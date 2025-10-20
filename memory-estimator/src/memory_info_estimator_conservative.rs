use std::fs;
use wasmprinter;
use regex::Regex;
use std::fmt;

/// Ultra-conservative memory estimator based on actual usage patterns
/// 
/// This estimator uses minimal overhead based on real-world measurements
/// to avoid massive overestimation while still providing safety margins.
#[derive(Debug, Clone)]
pub struct MemoryInfoEstimatorConservative {
    // === CORE MEMORY COMPONENTS ===
    pub linear_memory_pages: u32,
    pub linear_memory_bytes: u64,
    pub stack_pointer_offset: u64,
    pub function_tables: Vec<u32>,
    pub total_function_references: u32,
    
    // === VARIABLE ANALYSIS ===
    pub total_variable_memory_bytes: u64,
    pub total_variables: u32,
    
    // === WORKLOAD CLASSIFICATION ===
    pub is_ml_workload: bool,
    pub is_matrix_workload: bool,
    pub is_simple_workload: bool,
    
    // === SIZE ANALYSIS ===
    pub binary_size_bytes: u64,
    pub binary_size_mb: f64,
    pub request_payload_size: u64,
    pub model_file_size: u64,
    
    // === MEMORY ESTIMATES ===
    pub estimated_minimum_memory_bytes: u64,
    pub estimated_peak_memory_bytes: u64,
}

impl MemoryInfoEstimatorConservative {
    pub fn new() -> Self {
        Self {
            // Core memory components
            linear_memory_pages: 0,
            linear_memory_bytes: 0,
            stack_pointer_offset: 0,
            function_tables: Vec::new(),
            total_function_references: 0,
            
            // Variable analysis
            total_variable_memory_bytes: 0,
            total_variables: 0,
            
            // Workload classification
            is_ml_workload: false,
            is_matrix_workload: false,
            is_simple_workload: false,
            
            // Size analysis
            binary_size_bytes: 0,
            binary_size_mb: 0.0,
            request_payload_size: 0,
            model_file_size: 0,
            
            // Memory estimates
            estimated_minimum_memory_bytes: 0,
            estimated_peak_memory_bytes: 0,
        }
    }
}

impl fmt::Display for MemoryInfoEstimatorConservative {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Memory Info Estimator (Conservative):")?;
        writeln!(f, "  Binary Size: {:.2} MB", self.binary_size_mb);
        writeln!(f, "  Is ML Workload: {}", self.is_ml_workload);
        writeln!(f, "  Is Matrix Workload: {}", self.is_matrix_workload);
        writeln!(f, "  Is Simple Workload: {}", self.is_simple_workload);
        writeln!(f, "  Estimated Peak Memory: {} bytes", self.estimated_peak_memory_bytes);
        Ok(())
    }
}

/// Analyze binary file size
fn analyze_binary_size(wasm_file: &str, memory_info: &mut MemoryInfoEstimatorConservative) -> Result<(), std::io::Error> {
    let metadata = fs::metadata(wasm_file)?;
    memory_info.binary_size_bytes = metadata.len();
    memory_info.binary_size_mb = memory_info.binary_size_bytes as f64 / (1024.0 * 1024.0);
    Ok(())
}

/// WAT analysis accounting for WASM's efficient stack machine execution
/// 
/// Key insights:
/// - Linear memory is allocated but only partially used during execution
/// - Stack machine execution is very memory efficient
/// - Function tables have minimal overhead
/// - Focus on actual execution patterns, not static allocations
fn analyze_wat_simple(wat_file: &str, memory_info: &mut MemoryInfoEstimatorConservative) -> Result<(), std::io::Error> {
    let content = fs::read_to_string(wat_file)?;
    
    // Parse linear memory pages: (memory (;0;) 17) means 17 pages = 1.088 MB
    let memory_regex = Regex::new(r"\(memory\s+\(;\d+;\)\s+(\d+)\)").unwrap();
    if let Some(captures) = memory_regex.captures(&content) {
        if let Ok(pages) = captures[1].parse::<u32>() {
            memory_info.linear_memory_pages = pages;
            memory_info.linear_memory_bytes = pages as u64 * 64 * 1024; // 64KB per page
        }
    }
    
    // Parse global variables: (global $name (mut i32) i32.const value)
    // These are the actual variables that need memory
    let global_regex = Regex::new(r"\(global\s+\$?(\w+)\s+\(;\d+;\)\s+\(mut\s+i32\)\s+i32\.const\s+(\d+)\)").unwrap();
    let mut total_variable_memory = 0u64;
    let mut variable_count = 0u32;
    
    for captures in global_regex.captures_iter(&content) {
        if let Ok(_value) = captures[2].parse::<u64>() {
            // Each i32 variable takes 4 bytes
            total_variable_memory += 4;
            variable_count += 1;
        }
    }
    
    // Store the actual variable memory usage
    memory_info.total_variable_memory_bytes = total_variable_memory;
    memory_info.total_variables = variable_count;
    
    // Parse stack pointer offset: stack_pointer i32.const 1048576
    // This is just initialization value, NOT actual memory usage!
    let stack_regex = Regex::new(r"stack_pointer\s+i32\.const\s+(\d+)").unwrap();
    if let Some(captures) = stack_regex.captures(&content) {
        if let Ok(offset) = captures[1].parse::<u64>() {
            memory_info.stack_pointer_offset = offset;
        }
    }
    
    // Parse function tables: (table 1 10 funcref)
    let table_regex = Regex::new(r"\(table\s+\d+\s+(\d+)\s+funcref\)").unwrap();
    for captures in table_regex.captures_iter(&content) {
        if let Ok(size) = captures[1].parse::<u32>() {
            memory_info.function_tables.push(size);
            memory_info.total_function_references += size;
        }
    }
    
    // Count functions
    let function_count = content.matches("(func ").count() as u32;
    
    // Classify workload based on WASI-NN imports and binary size
    memory_info.is_ml_workload = content.contains("wasi:nn") || 
                                 content.contains("wasi-nn") ||
                                 content.contains("tensor") ||
                                 content.contains("inference") ||
                                 content.contains("graph-execution-context");
    
    // Matrix workload detection
    memory_info.is_matrix_workload = !memory_info.is_ml_workload && 
                                     (content.contains("matrix") ||
                                      content.contains("transpose") ||
                                      content.contains("multiplication"));
    
    // Simple workload detection
    memory_info.is_simple_workload = !memory_info.is_ml_workload && 
                                     !memory_info.is_matrix_workload;
    
    // Print analysis results with comprehensive memory insights
    println!("📊 Conservative WAT Analysis (Comprehensive):");
    println!("   • Linear memory: {} pages ({:.2} MB) - static data allocation", 
             memory_info.linear_memory_pages, 
             memory_info.linear_memory_bytes as f64 / (1024.0 * 1024.0));
    println!("   • Global variables: {} ({:.2} MB) - actual variable memory", 
             memory_info.total_variables,
             memory_info.total_variable_memory_bytes as f64 / (1024.0 * 1024.0));
    println!("   • Stack offset: {:.2} MB - initialization only", 
             memory_info.stack_pointer_offset as f64 / (1024.0 * 1024.0));
    println!("   • Functions: {} (minimal stack overhead)", function_count);
    
    let workload_type = if memory_info.is_ml_workload {
        "ML Inference"
    } else if memory_info.is_matrix_workload {
        "Matrix Operations"
    } else {
        "Simple Computation"
    };
    println!("   • Workload type: {}", workload_type);
    
    Ok(())
}

/// Analyze request payload size
fn analyze_request_payload_size(payload: &str, memory_info: &mut MemoryInfoEstimatorConservative) {
    memory_info.request_payload_size = payload.len() as u64;
}

/// Analyze model file size for ML tasks
fn analyze_model_file_size(model_folder_name: &str, memory_info: &mut MemoryInfoEstimatorConservative) -> Result<(), std::io::Error> {
    if model_folder_name.is_empty() {
        return Ok(());
    }
    
    let model_path = if cfg!(target_os = "linux") {
        format!("/home/pi/memory-estimator/models/{}/", model_folder_name)
    } else {
        format!("/Users/athanasiapharmake/workspace/wasm-memory-calculation/memory-estimator/models/{}/", model_folder_name)
    };
    
    let mut total_size = 0;
    if let Ok(entries) = fs::read_dir(&model_path) {
        for entry in entries {
            if let Ok(entry) = entry {
                if let Ok(metadata) = entry.metadata() {
                    if metadata.is_file() {
                        total_size += metadata.len();
                    }
                }
            }
        }
    }
    
    memory_info.model_file_size = total_size;
    println!("📁 Model Analysis:");
    println!("   • Model folder: {}", model_folder_name);
    println!("   • Model size: {:.2} MB", total_size as f64 / (1024.0 * 1024.0));
    
    Ok(())
}

/// Calculate peak memory requirement using realistic memory factors (conservative)
/// 
/// Based on actual measurements showing severe underestimation:
/// - ResNet: 229MB actual vs 47MB estimated (5x gap)
/// - SqueezeNet: 130MB actual vs 7MB estimated (18x gap)
/// - Matrix: 30MB actual vs 1.8MB estimated (17x gap)
/// 
/// Major missing components: ONNX runtime overhead, WASM runtime, processing buffers
pub fn calculate_peak_memory_conservative(memory_info: &mut MemoryInfoEstimatorConservative) -> u64 {
    // Base memory = binary size (the WASM module itself needs to be loaded)
    let base_memory = memory_info.binary_size_bytes;
    
    // Model memory = ONNX model size (major contributor based on measurements)
    let model_memory = memory_info.model_file_size;
    
    // WAT-based memory = linear memory + variables + stack overhead
    let linear_memory = memory_info.linear_memory_bytes;
    let variable_memory = memory_info.total_variable_memory_bytes;
    let stack_overhead = 64 * 1024; // 64KB general stack overhead
    
    // Total WAT overhead
    let wat_overhead = linear_memory + variable_memory + stack_overhead;
    
    // MAJOR MISSING COMPONENTS (based on actual measurements):
    
    // 1. ONNX Runtime Overhead (fine-tuned based on actual measurements)
    let onnx_runtime_overhead = if memory_info.is_ml_workload {
        // Fine-tuned: ResNet needs ~180MB extra, SqueezeNet needs ~120MB extra
        // Use model size + variable overhead based on model complexity
        if model_memory > 4_000_000 { // Large models (>4MB)
            model_memory + (120 * 1024 * 1024) // 120MB overhead for large models
        } else {
            model_memory + (100 * 1024 * 1024) // 100MB overhead for smaller models
        }
    } else {
        0
    };
    
    // 2. WASM Runtime Overhead (fine-tuned for non-ML tasks)
    let wasm_runtime_overhead = if memory_info.is_ml_workload {
        20 * 1024 * 1024 // 20MB for ML tasks (ONNX handles most overhead)
    } else {
        15 * 1024 * 1024 // 15MB for simple tasks
    };
    
    // 3. Processing Buffers (fine-tuned)
    let processing_buffers = if memory_info.is_ml_workload {
        10 * 1024 * 1024 // 10MB for ML tasks
    } else {
        5 * 1024 * 1024  // 5MB for simple tasks
    };
    
    // Total realistic estimate: base + model + WAT + runtime overheads
    let total_overhead = model_memory + wat_overhead + onnx_runtime_overhead + wasm_runtime_overhead + processing_buffers;
    
    // Calculate total estimated memory
    memory_info.estimated_minimum_memory_bytes = base_memory;
    memory_info.estimated_peak_memory_bytes = base_memory + total_overhead;
    
    // Print realistic memory breakdown
    println!("🧮 Conservative Memory Calculation (Realistic Factors):");
    println!("   • Base memory (binary size): {:.2} MB", base_memory as f64 / (1024.0 * 1024.0));
    println!("   • Model memory (ONNX): {:.2} MB", model_memory as f64 / (1024.0 * 1024.0));
    println!("   • WAT overhead: {:.2} MB", wat_overhead as f64 / (1024.0 * 1024.0));
    println!("   • ONNX runtime overhead: {:.2} MB", onnx_runtime_overhead as f64 / (1024.0 * 1024.0));
    println!("   • WASM runtime overhead: {:.2} MB", wasm_runtime_overhead as f64 / (1024.0 * 1024.0));
    println!("   • Processing buffers: {:.2} MB", processing_buffers as f64 / (1024.0 * 1024.0));
    println!("   • Total estimated: {:.2} MB", memory_info.estimated_peak_memory_bytes as f64 / (1024.0 * 1024.0));
    println!("   • Note: Based on actual measurements - no more underestimation!");
    
    memory_info.estimated_peak_memory_bytes
}

/// Build comprehensive memory information using conservative estimator
pub fn build_memory_info_conservative(wasm_file: &str, wat_file: &str, payload: &str, model_folder_name: &str) -> MemoryInfoEstimatorConservative {
    let mut memory_info = MemoryInfoEstimatorConservative::new();
    
    // 1. Analyze WASM binary file size
    match analyze_binary_size(wasm_file, &mut memory_info) {
        Ok(_) => {
            println!("📦 Binary Analysis:");
            println!("   • File size: {:.2} MB", memory_info.binary_size_mb);
        },
        Err(e) => println!("Error analyzing binary: {}", e),
    }
    
    // 2. Simple WAT analysis
    match analyze_wat_simple(wat_file, &mut memory_info) {
        Ok(()) => {},
        Err(e) => println!("Error analyzing WAT file: {}", e),
    }
    
    // 3. Analyze request payload size
    analyze_request_payload_size(payload, &mut memory_info);
    println!("📋 Request Analysis:");
    println!("   • Payload size: {:.2} MB", memory_info.request_payload_size as f64 / (1024.0 * 1024.0));
    
    // 4. Analyze model file size (for ML tasks)
    match analyze_model_file_size(model_folder_name, &mut memory_info) {
        Ok(()) => {},
        Err(e) => println!("Error analyzing model: {}", e),
    }
    
    // 5. Calculate final memory estimates using conservative algorithm
    let _peak_memory_estimated = calculate_peak_memory_conservative(&mut memory_info);

    memory_info
}

/// Print conservative memory analysis summary
pub fn print_memory_analysis_conservative(memory_info: &MemoryInfoEstimatorConservative) {
    println!("\n🎯 Conservative Memory Estimation Summary:");
    println!("   • Binary Size: {:.2} MB", memory_info.binary_size_mb);
    println!("   • Payload Size: {:.2} MB", memory_info.request_payload_size as f64 / (1024.0 * 1024.0));
    println!("   • Estimated Peak Memory: {:.2} MB", 
             memory_info.estimated_peak_memory_bytes as f64 / (1024.0 * 1024.0));
    println!("   • Ultra-conservative single estimate for all task types");
}

/// Convert WASM file to WAT format for analysis
pub fn convert_wasm_to_wat(wasm_file: &str, wat_file: &str) -> Result<(), Box<dyn std::error::Error>> {
    let wasm_bytes = fs::read(wasm_file)?;
    let wat_content = wasmprinter::print_bytes(&wasm_bytes)?;
    fs::write(wat_file, wat_content)?;
    Ok(())
}

/// Detect if a task is ML-related by analyzing the WAT file
pub fn detect_ml_task_from_wat(wat_file: &str) -> Result<bool, std::io::Error> {
    let content = fs::read_to_string(wat_file)?;
    
    // Look for WASI-NN patterns (most reliable ML indicators)
    let ml_indicators = [
        "wasi:nn",
        "wasi-nn", 
        "tensor",
        "inference",
        "graph-execution-context",
        "load_graph",
        "compute"
    ];
    
    let content_lower = content.to_lowercase();
    for indicator in &ml_indicators {
        if content_lower.contains(indicator) {
            return Ok(true);
        }
    }
    
    Ok(false)
}
