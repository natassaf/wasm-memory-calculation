use std::fs;
use wasmprinter;
use regex::Regex;
use std::fmt;

/// Improved memory estimator for WASM tasks based on actual usage patterns
/// 
/// This struct analyzes WASM files (.wasm/.wat) and estimates memory requirements
/// based on real-world measurements from Raspberry Pi testing.
#[derive(Debug, Clone)]
pub struct MemoryInfoEstimatorImproved {
    // === CORE MEMORY COMPONENTS ===
    /// Linear memory pages (each page = 64KB)
    pub linear_memory_pages: u32,
    
    /// Total linear memory in bytes (pages * 64KB)
    pub linear_memory_bytes: u64,
    
    /// Stack pointer offset in bytes
    pub stack_pointer_offset: u64,
    
    /// Function table sizes (for dynamic dispatch)
    pub function_tables: Vec<u32>,
    
    /// Total function references across all tables
    pub total_function_references: u32,
    
    // === FUNCTION COMPLEXITY ANALYSIS ===
    /// Maximum local variables in any single function
    pub max_local_variables_per_function: u32,
    
    /// Average local variables across all functions
    pub avg_local_variables_per_function: f32,
    
    /// Total local variables across all functions
    pub total_local_variables: u32,
    
    /// Functions with high local variable count (>10)
    pub high_complexity_functions: u32,
    
    // === WORKLOAD CLASSIFICATION ===
    /// True if this is a machine learning inference task
    /// Detected by: binary size > 600KB OR high function count + data sections
    pub is_ml_workload: bool,
    
    // === SIZE ANALYSIS ===
    /// Binary file size in bytes
    pub binary_size_bytes: u64,
    
    /// Binary file size in MB
    pub binary_size_mb: f64,
    
    /// Request payload size in bytes
    pub request_payload_size: u64,
    
    /// Model file size in bytes (for ML tasks)
    pub model_file_size: u64,
    
    // === MEMORY ESTIMATES ===
    /// Estimated minimum memory requirement in bytes
    pub estimated_minimum_memory_bytes: u64,
    
    /// Estimated peak memory requirement in bytes
    pub estimated_peak_memory_bytes: u64,
}

impl MemoryInfoEstimatorImproved {
    /// Create a new improved memory estimator with default values
    pub fn new() -> Self {
        Self {
            // Core memory components
            linear_memory_pages: 0,
            linear_memory_bytes: 0,
            stack_pointer_offset: 0,
            function_tables: Vec::new(),
            total_function_references: 0,
            
            // Function complexity analysis
            max_local_variables_per_function: 0,
            avg_local_variables_per_function: 0.0,
            total_local_variables: 0,
            high_complexity_functions: 0,
            
            // Workload classification
            is_ml_workload: false,
            
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

impl fmt::Display for MemoryInfoEstimatorImproved {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Memory Info Estimator (Improved):")?;
        writeln!(f, "  Linear Memory: {} pages ({} bytes)", self.linear_memory_pages, self.linear_memory_bytes)?;
        writeln!(f, "  Stack Pointer Offset: {} bytes", self.stack_pointer_offset)?;
        writeln!(f, "  Function Tables: {:?}", self.function_tables)?;
        writeln!(f, "  Total Function References: {}", self.total_function_references)?;
        writeln!(f, "  Max Local Variables per Function: {}", self.max_local_variables_per_function)?;
        writeln!(f, "  Average Local Variables per Function: {:.2}", self.avg_local_variables_per_function)?;
        writeln!(f, "  Total Local Variables: {}", self.total_local_variables)?;
        writeln!(f, "  High Complexity Functions: {}", self.high_complexity_functions)?;
        writeln!(f, "  Is ML Workload: {}", self.is_ml_workload)?;
        writeln!(f, "  Binary Size: {} bytes ({:.2} MB)", self.binary_size_bytes, self.binary_size_mb)?;
        writeln!(f, "  Request Payload Size: {} bytes", self.request_payload_size)?;
        writeln!(f, "  Model File Size: {} bytes", self.model_file_size)?;
        writeln!(f, "  Estimated Minimum Memory: {} bytes", self.estimated_minimum_memory_bytes)?;
        writeln!(f, "  Estimated Peak Memory: {} bytes", self.estimated_peak_memory_bytes)?;
        Ok(())
    }
}

/// Analyze binary file size and categorize it
fn analyze_binary_size(wasm_file: &str, memory_info: &mut MemoryInfoEstimatorImproved) -> Result<(), std::io::Error> {
    let metadata = fs::metadata(wasm_file)?;
    memory_info.binary_size_bytes = metadata.len();
    memory_info.binary_size_mb = memory_info.binary_size_bytes as f64 / (1024.0 * 1024.0);
    Ok(())
}

/// Categorize binary size for workload detection
fn categorize_binary_size(size_bytes: u64) -> &'static str {
    if size_bytes > 600_000 {
        "Large (ML workload)"
    } else if size_bytes > 150_000 {
        "Medium (Matrix workload)"
    } else {
        "Small (Simple workload)"
    }
}

/// Analyze WAT file for memory requirements and workload classification
fn analyze_wat_memory_simple(wat_file: &str, memory_info: &mut MemoryInfoEstimatorImproved) -> Result<(), std::io::Error> {
    let content = fs::read_to_string(wat_file)?;
    
    // Parse linear memory pages: (memory 1 2) means 2 pages = 128KB
    let memory_regex = Regex::new(r"\(memory\s+\d+\s+(\d+)\)").unwrap();
    if let Some(captures) = memory_regex.captures(&content) {
        if let Ok(pages) = captures[1].parse::<u32>() {
            memory_info.linear_memory_pages = pages;
            memory_info.linear_memory_bytes = pages as u64 * 64 * 1024; // 64KB per page
        }
    }
    
    // Parse stack pointer offset: stack_pointer i32.const 1048576
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
    
    // Analyze function complexity by counting local variables
    let mut function_count = 0;
    let mut total_locals = 0;
    let mut max_locals = 0;
    let mut high_complexity_count = 0;
    
    // Count functions and their local variables
    let func_regex = Regex::new(r"\(func[^)]*\)").unwrap();
    for func_match in func_regex.find_iter(&content) {
        function_count += 1;
        let func_content = &content[func_match.start()..func_match.end()];
        
        // Count local variables in this function
        let local_count = func_content.matches("(local ").count() as u32;
        total_locals += local_count;
        max_locals = max_locals.max(local_count);
        
        if local_count > 10 {
            high_complexity_count += 1;
        }
    }
    
    memory_info.max_local_variables_per_function = max_locals;
    memory_info.total_local_variables = total_locals;
    memory_info.high_complexity_functions = high_complexity_count;
    
    if function_count > 0 {
        memory_info.avg_local_variables_per_function = total_locals as f32 / function_count as f32;
    }
    
    // Classify workload based on binary size and complexity
    memory_info.is_ml_workload = memory_info.binary_size_bytes > 600_000 || 
                                 (function_count > 500 && memory_info.binary_size_bytes > 200_000);
    
    // Print analysis results
    println!("📊 WAT Analysis:");
    println!("   • Linear memory: {} pages ({:.2} MB)", 
             memory_info.linear_memory_pages, 
             memory_info.linear_memory_bytes as f64 / (1024.0 * 1024.0));
    println!("   • Stack offset: {:.2} MB", 
             memory_info.stack_pointer_offset as f64 / (1024.0 * 1024.0));
    println!("   • Function tables: {} total references", memory_info.total_function_references);
    println!("   • Function complexity: {} max locals, {} avg locals, {} high-complexity functions", 
             memory_info.max_local_variables_per_function,
             memory_info.avg_local_variables_per_function,
             memory_info.high_complexity_functions);
    
    let workload_type = if memory_info.is_ml_workload {
        "ML Inference"
    } else {
        "Non ML"
    };
    println!("   • Workload type: {}", workload_type);
    
    Ok(())
}

/// Analyze request payload size
fn analyze_request_payload_size(payload: &str, memory_info: &mut MemoryInfoEstimatorImproved) {
    memory_info.request_payload_size = payload.len() as u64;
}

/// Analyze model file size for ML tasks
fn analyze_model_file_size(model_folder_name: &str, memory_info: &mut MemoryInfoEstimatorImproved) -> Result<(), std::io::Error> {
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

/// Calculate peak memory requirement based on actual usage patterns
/// 
/// Improved estimator based on real-world measurements from Raspberry Pi:
/// - ML tasks: ~0.13-0.23 MB actual → ~4 MB estimated (20x safety margin)
/// - Non-ML tasks: ~0.03 MB actual → ~2 MB estimated (70x safety margin)
/// 
/// This estimator provides conservative estimates that match observed patterns
/// while maintaining reasonable safety margins for runtime overhead.
pub fn calculate_peak_memory_improved(memory_info: &mut MemoryInfoEstimatorImproved) -> u64 {
    // Base memory = binary size (the WASM module itself needs to be loaded)
    let base_memory = memory_info.binary_size_bytes;
    
    // More accurate overhead calculation based on actual measurements
    let runtime_overhead = if memory_info.is_ml_workload {
        // ML tasks: Actual ~0.13-0.23 MB, use 20x multiplier for safety
        // This gives us ~2.6-4.6 MB estimated (close to our 4.1 MB results)
        4 * 1024 * 1024  // 4MB for ML tasks
    } else {
        // Non-ML tasks: Actual ~0.03 MB, use 70x multiplier for safety  
        // This gives us ~2.1 MB estimated (matching our results)
        2 * 1024 * 1024  // 2MB for non-ML tasks
    };
    
    // Calculate total estimated memory
    memory_info.estimated_minimum_memory_bytes = base_memory;
    memory_info.estimated_peak_memory_bytes = base_memory + runtime_overhead;
    
    // Print simplified memory breakdown
    println!("🧮 Memory Calculation (Improved - Binary-Based):");
    println!("   • Base memory (binary size): {:.2} MB", base_memory as f64 / (1024.0 * 1024.0));
    println!("   • Workload overhead: {:.2} MB", runtime_overhead as f64 / (1024.0 * 1024.0));
    println!("   • Total estimated: {:.2} MB", memory_info.estimated_peak_memory_bytes as f64 / (1024.0 * 1024.0));
    
    memory_info.estimated_peak_memory_bytes
}

/// Build comprehensive memory information for a WASM task using improved estimator
/// 
/// This function analyzes a WASM task and provides accurate memory estimates
/// based on real-world usage patterns observed on Raspberry Pi.
pub fn build_memory_info_improved(wasm_file: &str, wat_file: &str, payload: &str, model_folder_name: &str) -> MemoryInfoEstimatorImproved {
    let mut memory_info = MemoryInfoEstimatorImproved::new();
    
    // 1. Analyze WASM binary file size
    match analyze_binary_size(wasm_file, &mut memory_info) {
        Ok(_) => {
            let size_category = categorize_binary_size(memory_info.binary_size_bytes);
            println!("📦 Binary Analysis:");
            println!("   • File size: {:.2} MB", memory_info.binary_size_mb);
            println!("   • Size category: {}", size_category);
        },
        Err(e) => println!("Error analyzing binary: {}", e),
    }
    
    // 2. Analyze WAT file memory requirements
    match analyze_wat_memory_simple(wat_file, &mut memory_info) {
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
    
    // 5. Calculate final memory estimates using improved algorithm
    let _peak_memory_estimated = calculate_peak_memory_improved(&mut memory_info);

    memory_info
}

/// Print simplified memory analysis summary for improved estimator
pub fn print_memory_analysis_improved(memory_info: &MemoryInfoEstimatorImproved) {
    println!("\n🎯 Memory Estimation Summary (Improved):");
    println!("   • Binary Size: {:.2} MB", memory_info.binary_size_mb);
    println!("   • Payload Size: {:.2} MB", memory_info.request_payload_size as f64 / (1024.0 * 1024.0));
    println!("   • Workload Type: {}", if memory_info.is_ml_workload { "ML Inference" } else { "Non ML" });
    println!("   • Estimated Peak Memory: {:.2} MB", 
             memory_info.estimated_peak_memory_bytes as f64 / (1024.0 * 1024.0));
    
    if memory_info.is_ml_workload {
        println!("   • ML task detected - estimated 4MB overhead for ONNX runtime");
    } else {
        println!("   • Non-ML task detected - estimated 2MB overhead for basic runtime");
    }
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
    
    // Look for ML-related patterns in the WAT file
    let ml_indicators = [
        "onnx", "tensor", "model", "inference", "convolution", 
        "matrix", "neural", "deep", "learning", "classification"
    ];
    
    let content_lower = content.to_lowercase();
    for indicator in &ml_indicators {
        if content_lower.contains(indicator) {
            return Ok(true);
        }
    }
    
    // Also check for large data sections which are common in ML models
    let data_section_count = content.matches("(data ").count();
    if data_section_count > 10 {
        return Ok(true);
    }
    
    Ok(false)
}
