use std::fs;
use wasmprinter;
use regex::Regex;
use std::fmt;

/// Simplified memory estimator for WASM tasks
/// 
/// This struct analyzes WASM files (.wasm/.wat) and estimates memory requirements
/// based on:
/// - Binary size (indicates complexity)
/// - Linear memory pages (static memory allocation)
/// - Stack pointer offset (runtime stack space)
/// - Function count (code complexity)
/// - Request payload size (input data size)
/// - Model file size (for ML tasks)
#[derive(Debug, Clone)]
pub struct MemoryInfoEstimator {
    // === CORE MEMORY COMPONENTS ===
    /// Linear memory pages (each page = 64KB)
    /// Found in .wat file: (memory 1 2) means 2 pages = 128KB
    pub linear_memory_pages: u32,
    
    /// Total linear memory in bytes (pages * 64KB)
    /// This is the static memory allocated by the WASM module
    pub linear_memory_bytes: u64,
    
    /// Stack pointer offset in bytes
    /// Found in .wat file: stack_pointer i32.const 1048576
    /// This is runtime stack space needed
    pub stack_pointer_offset: u64,
    
    /// Function table sizes (for dynamic dispatch)
    /// Found in .wat file: (table 1 10 funcref)
    pub function_tables: Vec<u32>,
    
    /// Total function references across all tables
    pub total_function_references: u32,
    
    // === FUNCTION COMPLEXITY ANALYSIS ===
    /// Maximum local variables in any single function
    /// Higher values indicate more complex algorithms (e.g., memoization)
    pub max_local_variables_per_function: u32,
    
    /// Average local variables across all functions
    /// Indicates overall memory usage pattern
    pub avg_local_variables_per_function: f32,
    
    /// Total local variables across all functions
    /// Used for memory estimation calculations
    pub total_local_variables: u32,
    
    /// Functions with high local variable count (>10)
    /// Indicates memory-intensive algorithms
    pub high_complexity_functions: u32,
    
    // === WORKLOAD CLASSIFICATION ===
    /// True if this is a machine learning inference task
    /// Detected by: binary size > 600KB OR high function count + data sections
    pub is_ml_workload: bool,
    
    // === SIZE ANALYSIS ===
    /// WASM binary file size in bytes
    /// Larger files typically need more memory
    pub binary_size_bytes: u64,
    
    /// WASM binary file size in MB (for readability)
    pub binary_size_mb: f64,
    
    /// Request payload size in bytes
    /// Larger payloads need more memory for processing
    pub request_payload_size: u64,
    
    /// Model file size in bytes (for ML tasks)
    /// Found in models/ folder, affects memory for model loading
    pub model_file_size: u64,
    
    // === MEMORY ESTIMATES ===
    /// Minimum memory needed (base memory + overhead)
    pub estimated_minimum_memory_bytes: u64,
    
    /// Peak memory needed (minimum + dynamic buffers)
    pub estimated_peak_memory_bytes: u64,
}
impl MemoryInfoEstimator {
    /// Create a new memory estimator with default values
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


impl fmt::Display for MemoryInfoEstimator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MemoryInfo:\n\
             - linear_memory_pages: {} ({} MB)\n\
             - stack_pointer_offset: {} bytes ({:.2} MB)\n\
             - function_tables: {:?}\n\
             - total_function_references: {}\n\
             - binary_size: {:.2} MB\n\
             - request_payload: {:.2} MB\n\
             - model_file: {:.2} MB\n\
             - workload_type: {}\n\
             - estimated_minimum: {:.2} MB\n\
             - estimated_peak: {:.2} MB",
            self.linear_memory_pages,
            self.linear_memory_bytes as f64 / (1024.0 * 1024.0),
            self.stack_pointer_offset,
            self.stack_pointer_offset as f64 / (1024.0 * 1024.0),
            self.function_tables,
            self.total_function_references,
            self.binary_size_mb,
            self.request_payload_size as f64 / (1024.0 * 1024.0),
            self.model_file_size as f64 / (1024.0 * 1024.0),
            if self.is_ml_workload { "ML" } else { "Non ML" },
            self.estimated_minimum_memory_bytes as f64 / (1024.0 * 1024.0),
            self.estimated_peak_memory_bytes as f64 / (1024.0 * 1024.0),
        )
    }
}


/// Analyze WASM binary file size
/// 
/// This function reads the WASM file metadata to determine:
/// - Binary size in bytes (indicates code complexity)
/// - Binary size in MB (for human readability)
/// 
/// Larger binaries typically need more memory for:
/// - Code loading and execution
/// - Runtime overhead
/// - Function dispatch tables
pub fn analyze_binary_size(wasm_path: &str, memory_info: &mut MemoryInfoEstimator) -> Result<(), Box<dyn std::error::Error>> {
    let metadata = fs::metadata(wasm_path)?;
    let binary_size_bytes = metadata.len();
    let binary_size_mb = binary_size_bytes as f64 / (1024.0 * 1024.0);
    memory_info.binary_size_bytes = binary_size_bytes;
    memory_info.binary_size_mb = binary_size_mb;
    Ok(())
}

/// Analyze request payload size
/// 
/// This function calculates the size of the input data that will be processed.
/// Larger payloads need more memory for:
/// - Input data storage
/// - Processing buffers
/// - Intermediate results
pub fn analyze_request_payload_size(payload: &str, memory_info: &mut MemoryInfoEstimator) {
    memory_info.request_payload_size = payload.len() as u64;
}

/// Analyze model file size for ML tasks
/// 
/// This function checks the models/ folder for the specified model and calculates:
/// - Model file size in bytes (affects memory for model loading)
/// - Model loading overhead (typically 2-3x model size)
/// 
/// For ML tasks, the model size is crucial for memory estimation because:
/// - Models are loaded into memory during inference
/// - ONNX Runtime needs additional memory for model operations
/// - Larger models (ResNet, etc.) need significantly more memory than smaller ones (SqueezeNet)
pub fn analyze_model_file_size(model_folder_name: &str, memory_info: &mut MemoryInfoEstimator) -> Result<(), Box<dyn std::error::Error>> {
    // Look for ONNX model files in the models folder
    let model_path = format!("models/{}/", model_folder_name);
    
    if let Ok(entries) = fs::read_dir(&model_path) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if let Some(extension) = path.extension() {
                    if extension == "onnx" {
                        let metadata = fs::metadata(&path)?;
                        memory_info.model_file_size = metadata.len();
                        println!("📦 Model Analysis:");
                        println!("   • Model file: {}", path.display());
                        println!("   • Model size: {:.2} MB", memory_info.model_file_size as f64 / (1024.0 * 1024.0));
                        break;
                    }
                }
            }
        }
    }
    
    Ok(())
}

/// Categorize binary size
pub fn categorize_binary_size(size_bytes: u64) -> &'static str {
    match size_bytes {
        0..=50_000 => "Tiny (< 50KB)",
        50_001..=100_000 => "Small (50-100KB)",
        100_001..=200_000 => "Medium (100-200KB)",
        200_001..=500_000 => "Large (200-500KB)",
        500_001..=1_000_000 => "Very Large (500KB-1MB)",
        _ => "Huge (> 1MB)"
    }
}


/// Convert a .wasm file to .wat format using wasmprinter crate
pub fn convert_wasm_to_wat(wasm_path: &str, wat_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let wasm_bytes = fs::read(wasm_path)?;
    let wat_string = wasmprinter::print_bytes(&wasm_bytes)?;
    fs::write(wat_path, wat_string)?;
    Ok(())
}

/// Detect if a WASM task is a Machine Learning task by analyzing WASI-NN usage
/// 
/// This function looks for the most essential WASI-NN operations:
/// - wasi-nn imports
/// - Graph loading operations
/// - Tensor operations
/// 
/// Returns true if ML operations are detected, false otherwise.
pub fn detect_ml_task_from_wat(wat_path: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(wat_path)?;
    
    // Look for essential WASI-NN patterns only
    let wasi_nn_patterns = [
        // WASI-NN imports (most reliable indicator)
        r"\(import.*wasi-nn",
        r"\(import.*wasi::nn",
        
        // Core WASI-NN operations
        r"load_graph",
        r"load_tensor",
        r"compute",
    ];
    
    let mut ml_indicators = 0;
    
    for pattern in &wasi_nn_patterns {
        let regex = Regex::new(pattern)?;
        let matches = regex.find_iter(&content).count();
        if matches > 0 {
            ml_indicators += matches;
            println!("   • Found {} ML indicator(s): {}", matches, pattern);
        }
    }
    
    let is_ml_task = ml_indicators > 0;
    
    println!("📊 ML Task Detection:");
    println!("   • Total ML indicators found: {}", ml_indicators);
    println!("   • Is ML task: {}", if is_ml_task { "Yes" } else { "No" });
    
    Ok(is_ml_task)
}

/// Analyze local variables in WAT functions
/// 
/// This function extracts local variable information from function declarations:
/// - Counts local variables per function: (local i32 i32 i32) = 3 locals
/// - Identifies high-complexity functions (>10 locals)
/// - Calculates statistics for memory estimation
/// 
/// Higher local variable counts indicate memory-intensive algorithms like memoization.
pub fn analyze_local_variables(wat_path: &str, memory_info: &mut MemoryInfoEstimator) -> Result<(), Box<dyn std::error::Error>> {
    let content = fs::read_to_string(wat_path)?;
    
    // Find all local variable declarations in functions
    // Pattern matches: (local i32 i32 i32) or (local i32)
    let local_regex = Regex::new(r"\(local\s+([^)]+)\)")?;
    
    let mut function_local_counts = Vec::new();
    let mut total_locals = 0u32;
    let mut high_complexity_count = 0u32;
    let mut max_locals_in_function = 0u32;
    
    // Find all local variable declarations
    for local_match in local_regex.find_iter(&content) {
        let local_text = &local_match.as_str()[7..local_match.as_str().len()-1]; // Remove "(local " and ")"
        let local_count = local_text.split_whitespace().count() as u32;
        
        function_local_counts.push(local_count);
        total_locals += local_count;
        max_locals_in_function = max_locals_in_function.max(local_count);
        
        // Count high-complexity functions (>10 locals)
        if local_count > 10 {
            high_complexity_count += 1;
        }
    }
    
    // Calculate statistics
    if !function_local_counts.is_empty() {
        memory_info.max_local_variables_per_function = max_locals_in_function;
        memory_info.avg_local_variables_per_function = total_locals as f32 / function_local_counts.len() as f32;
        memory_info.total_local_variables = total_locals;
        memory_info.high_complexity_functions = high_complexity_count;
    }
    
    println!("🔍 Local Variable Analysis:");
    println!("   • Max locals per function: {}", memory_info.max_local_variables_per_function);
    println!("   • Average locals per function: {:.1}", memory_info.avg_local_variables_per_function);
    println!("   • Total local variables: {}", memory_info.total_local_variables);
    println!("   • High-complexity functions (>10 locals): {}", memory_info.high_complexity_functions);
    
    Ok(())
}

/// Analyze memory requirements from a .wat file
/// 
/// This function parses the WAT (WebAssembly Text) file to extract:
/// - Linear memory pages: (memory 1 2) = 2 pages = 128KB
/// - Stack pointer offset: stack_pointer i32.const 1048576 = 1MB stack
/// - Function tables: (table 1 10 funcref) = 10 function references
/// - Function count: Number of (func ...) declarations
/// - Data sections: Number of (data ...) declarations
/// - Global variables: Number of (global ...) declarations
/// - Local variables: Analysis of function complexity
/// 
/// These values help classify the workload and estimate memory needs.
pub fn analyze_wat_memory_simple(wat_path: &str, memory_info: &mut MemoryInfoEstimator) -> Result<(), Box<dyn std::error::Error>> {
    let content = fs::read_to_string(wat_path)?;
    
    // Regex patterns to extract memory information from WAT file
    let memory_regex = Regex::new(r"\(memory.*?(\d+)\)")?;           // (memory 1 2) -> 2 pages
    let stack_regex = Regex::new(r"stack_pointer.*?i32\.const\s+(\d+)")?;  // stack_pointer i32.const 1048576
    let table_regex = Regex::new(r"\(table.*?(\d+).*?(\d+).*?funcref\)")?; // (table 1 10 funcref) -> 10 refs
    
    // Extract linear memory pages (each page = 64KB)
    if let Some(caps) = memory_regex.captures(&content) {
        memory_info.linear_memory_pages = caps[1].parse()?;
        memory_info.linear_memory_bytes = memory_info.linear_memory_pages as u64 * 65536; // 64KB per page
    }
    
    // Extract stack pointer offset (runtime stack space)
    if let Some(caps) = stack_regex.captures(&content) {
        memory_info.stack_pointer_offset = caps[1].parse()?;
    }
    
    // Extract function table sizes (for dynamic dispatch)
    for caps in table_regex.captures_iter(&content) {
        let table_size = caps[1].parse::<u32>()?;
        memory_info.function_tables.push(table_size);
    }
    memory_info.total_function_references = memory_info.function_tables.iter().sum();
    
    // Count code complexity indicators
    let function_count = content.lines().filter(|line| line.trim().starts_with("(func ")).count();
    let data_section_count = content.lines().filter(|line| line.trim().starts_with("(data ")).count();
    let global_count = content.lines().filter(|line| line.trim().starts_with("(global ")).count();
    
    // Analyze local variables for function complexity
    analyze_local_variables(wat_path, memory_info)?;
    
    // Detect ML task using WASI-NN analysis instead of binary size
    memory_info.is_ml_workload = match detect_ml_task_from_wat(wat_path) {
        Ok(is_ml) => is_ml,
        Err(e) => {
            println!("Error detecting ML task: {}", e);
            false
        }
    };
    

    // Print analysis results
    println!("📊 WAT Analysis:");
    println!("   • Functions: {}", function_count);
    println!("   • Data sections: {}", data_section_count);
    println!("   • Globals: {}", global_count);
    println!("   • Linear memory: {} pages ({:.2} MB)", 
             memory_info.linear_memory_pages,
             memory_info.linear_memory_bytes as f64 / (1024.0 * 1024.0));
    println!("   • Stack space: {:.2} MB", 
             memory_info.stack_pointer_offset as f64 / (1024.0 * 1024.0));
    
    let workload_type = if memory_info.is_ml_workload {
        "ML Inference"
    } else {
        "Non ML"
    };
    println!("   • Workload type: {}", workload_type);
    
    Ok(())
}

/// Calculate memory estimates based on analysis
/// 
/// This function combines all analyzed data to estimate:
/// - Minimum memory: Base memory + binary overhead
/// Calculate peak memory requirement based on actual usage patterns
/// 
/// Simplified estimator based on real-world measurements:
/// - ML tasks: ~0.1-0.3 MB actual usage
/// - Matrix operations: ~0.03 MB actual usage  
/// - Simple tasks: ~0.03 MB actual usage
/// 
/// This estimator uses a conservative 3MB overhead for all task types
/// to account for runtime overhead while keeping estimates reasonable.
pub fn calculate_peak_memory(memory_info: &mut MemoryInfoEstimator) -> u64 {
    // Base memory = static memory + stack space
    let base_memory = memory_info.linear_memory_bytes + memory_info.stack_pointer_offset;
    
    // Simple overhead calculation based on workload type and actual measurements
    let runtime_overhead = if memory_info.is_ml_workload {
        // ML tasks: Based on actual measurements (~0.1-0.3 MB)
        // Use conservative 10x multiplier for safety
        3 * 1024 * 1024  // 3MB for ML tasks
    }else{
        0
    };
    
    // Calculate total estimated memory
    memory_info.estimated_minimum_memory_bytes = base_memory;
    memory_info.estimated_peak_memory_bytes = base_memory + runtime_overhead;
    
    // Print simplified memory breakdown
    println!("🧮 Memory Calculation:");
    println!("   • Base memory: {:.2} MB", base_memory as f64 / (1024.0 * 1024.0));
    println!("   • Runtime overhead: {:.2} MB", runtime_overhead as f64 / (1024.0 * 1024.0));
    println!("   • Total estimated: {:.2} MB", memory_info.estimated_peak_memory_bytes as f64 / (1024.0 * 1024.0));
    
    memory_info.estimated_peak_memory_bytes
}


/// Build comprehensive memory information for a WASM task
/// 
/// This function analyzes all components needed for memory estimation:
/// 1. WASM binary file size and complexity
/// 2. WAT file memory requirements (linear memory, stack, functions)
/// 3. Request payload size (input data)
/// 4. Model file size (for ML tasks)
/// 
/// Returns a complete MemoryInfoEstimator with all estimates calculated.
pub fn build_memory_info(wasm_file: &str, wat_file: &str, payload: &str, model_folder_name: &str) -> MemoryInfoEstimator {
    let mut memory_info = MemoryInfoEstimator::new();
    
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
    
    // 5. Calculate final memory estimates
    let peak_memory_estimated = calculate_peak_memory(&mut memory_info);

    memory_info
}

/// Print simplified memory analysis summary
/// 
/// This function provides a clean summary of the memory analysis results
/// with clear recommendations for task allocation.
pub fn print_memory_analysis_simple(memory_info: &MemoryInfoEstimator) {
    println!("\n🎯 Memory Estimation Summary:");
    println!("   • Binary Size: {:.2} MB", memory_info.binary_size_mb);
    println!("   • Payload Size: {:.2} MB", memory_info.request_payload_size as f64 / (1024.0 * 1024.0));
    println!("   • Model Size: {:.2} MB", memory_info.model_file_size as f64 / (1024.0 * 1024.0));
    
    println!("\n💾 Memory Requirements:");
    println!("   • Minimum Memory: {:.2} MB", memory_info.estimated_minimum_memory_bytes as f64 / (1024.0 * 1024.0));
    println!("   • Peak Memory: {:.2} MB", memory_info.estimated_peak_memory_bytes as f64 / (1024.0 * 1024.0));
    
    println!("\n📋 Allocation Recommendation:");
    println!("   • Allocate at least {:.2} MB for safe execution",
             memory_info.estimated_peak_memory_bytes as f64 / (1024.0 * 1024.0));
    
}
