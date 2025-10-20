use std::fs;
use wasmprinter;
use regex::Regex;
use std::fmt;

/// Advanced memory estimator for WASM tasks based on comprehensive WAT analysis
/// 
/// This estimator uses multiple WAT fields and patterns to create more accurate
/// memory predictions without overfitting to specific test cases.
#[derive(Debug, Clone)]
pub struct MemoryInfoEstimatorAdvanced {
    // === CORE MEMORY COMPONENTS ===
    pub linear_memory_pages: u32,
    pub linear_memory_bytes: u64,
    pub stack_pointer_offset: u64,
    pub function_tables: Vec<u32>,
    pub total_function_references: u32,
    
    // === ADVANCED WAT ANALYSIS ===
    /// Number of imports (indicates external dependencies)
    pub import_count: u32,
    
    /// Number of exports (indicates interface complexity)
    pub export_count: u32,
    
    /// Number of data sections (indicates static data size)
    pub data_section_count: u32,
    
    /// Number of global variables (indicates state management)
    pub global_variable_count: u32,
    
    /// Total size of data sections in bytes
    pub data_section_size_bytes: u64,
    
    /// Number of type definitions (indicates interface complexity)
    pub type_definition_count: u32,
    
    /// Number of instance definitions (indicates component complexity)
    pub instance_count: u32,
    
    /// Number of resource definitions (indicates memory management)
    pub resource_count: u32,
    
    // === FUNCTION COMPLEXITY ANALYSIS ===
    pub max_local_variables_per_function: u32,
    pub avg_local_variables_per_function: f32,
    pub total_local_variables: u32,
    pub high_complexity_functions: u32,
    pub function_count: u32,
    
    // === WORKLOAD CLASSIFICATION ===
    /// ML workload detection based on WASI-NN imports
    pub is_ml_workload: bool,
    
    /// Matrix workload detection based on patterns
    pub is_matrix_workload: bool,
    
    /// Simple workload detection
    pub is_simple_workload: bool,
    
    /// Component complexity score (0-100)
    pub complexity_score: u32,
    
    // === SIZE ANALYSIS ===
    pub binary_size_bytes: u64,
    pub binary_size_mb: f64,
    pub request_payload_size: u64,
    pub model_file_size: u64,
    
    // === MEMORY ESTIMATES ===
    pub estimated_minimum_memory_bytes: u64,
    pub estimated_peak_memory_bytes: u64,
}

impl MemoryInfoEstimatorAdvanced {
    pub fn new() -> Self {
        Self {
            // Core memory components
            linear_memory_pages: 0,
            linear_memory_bytes: 0,
            stack_pointer_offset: 0,
            function_tables: Vec::new(),
            total_function_references: 0,
            
            // Advanced WAT analysis
            import_count: 0,
            export_count: 0,
            data_section_count: 0,
            global_variable_count: 0,
            data_section_size_bytes: 0,
            type_definition_count: 0,
            instance_count: 0,
            resource_count: 0,
            
            // Function complexity analysis
            max_local_variables_per_function: 0,
            avg_local_variables_per_function: 0.0,
            total_local_variables: 0,
            high_complexity_functions: 0,
            function_count: 0,
            
            // Workload classification
            is_ml_workload: false,
            is_matrix_workload: false,
            is_simple_workload: false,
            complexity_score: 0,
            
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

impl fmt::Display for MemoryInfoEstimatorAdvanced {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Memory Info Estimator (Advanced):")?;
        writeln!(f, "  Linear Memory: {} pages ({} bytes)", self.linear_memory_pages, self.linear_memory_bytes)?;
        writeln!(f, "  Stack Pointer Offset: {} bytes", self.stack_pointer_offset)?;
        writeln!(f, "  Imports: {}", self.import_count)?;
        writeln!(f, "  Exports: {}", self.export_count)?;
        writeln!(f, "  Data Sections: {} ({} bytes)", self.data_section_count, self.data_section_size_bytes)?;
        writeln!(f, "  Global Variables: {}", self.global_variable_count)?;
        writeln!(f, "  Type Definitions: {}", self.type_definition_count)?;
        writeln!(f, "  Instances: {}", self.instance_count)?;
        writeln!(f, "  Resources: {}", self.resource_count)?;
        writeln!(f, "  Functions: {}", self.function_count)?;
        writeln!(f, "  Complexity Score: {}", self.complexity_score)?;
        writeln!(f, "  Is ML Workload: {}", self.is_ml_workload)?;
        writeln!(f, "  Is Matrix Workload: {}", self.is_matrix_workload)?;
        writeln!(f, "  Is Simple Workload: {}", self.is_simple_workload)?;
        writeln!(f, "  Estimated Peak Memory: {} bytes", self.estimated_peak_memory_bytes)?;
        Ok(())
    }
}

/// Analyze binary file size
fn analyze_binary_size(wasm_file: &str, memory_info: &mut MemoryInfoEstimatorAdvanced) -> Result<(), std::io::Error> {
    let metadata = fs::metadata(wasm_file)?;
    memory_info.binary_size_bytes = metadata.len();
    memory_info.binary_size_mb = memory_info.binary_size_bytes as f64 / (1024.0 * 1024.0);
    Ok(())
}

/// Advanced WAT analysis using multiple fields and patterns
fn analyze_wat_advanced(wat_file: &str, memory_info: &mut MemoryInfoEstimatorAdvanced) -> Result<(), std::io::Error> {
    let content = fs::read_to_string(wat_file)?;
    
    // === BASIC MEMORY ANALYSIS ===
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
    
    // === ADVANCED COMPONENT ANALYSIS ===
    // Count imports (external dependencies)
    memory_info.import_count = content.matches("(import ").count() as u32;
    
    // Count exports (interface complexity)
    memory_info.export_count = content.matches("(export ").count() as u32;
    
    // Count data sections (static data)
    memory_info.data_section_count = content.matches("(data ").count() as u32;
    
    // Count global variables (state management)
    memory_info.global_variable_count = content.matches("(global ").count() as u32;
    
    // Count type definitions (interface complexity)
    memory_info.type_definition_count = content.matches("(type ").count() as u32;
    
    // Count instance definitions (component complexity)
    memory_info.instance_count = content.matches("(instance ").count() as u32;
    
    // Count resource definitions (memory management)
    memory_info.resource_count = content.matches("(resource ").count() as u32;
    
    // Estimate data section size (rough approximation)
    let data_size_regex = Regex::new(r"\(data[^)]*\)").unwrap();
    for data_match in data_size_regex.find_iter(&content) {
        let data_content = &content[data_match.start()..data_match.end()];
        // Rough estimate: count characters in data section
        memory_info.data_section_size_bytes += data_content.len() as u64;
    }
    
    // === FUNCTION COMPLEXITY ANALYSIS ===
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
    
    memory_info.function_count = function_count;
    memory_info.max_local_variables_per_function = max_locals;
    memory_info.total_local_variables = total_locals;
    memory_info.high_complexity_functions = high_complexity_count;
    
    if function_count > 0 {
        memory_info.avg_local_variables_per_function = total_locals as f32 / function_count as f32;
    }
    
    // === WORKLOAD CLASSIFICATION ===
    // ML workload detection based on WASI-NN imports
    memory_info.is_ml_workload = content.contains("wasi:nn") || 
                                 content.contains("wasi-nn") ||
                                 content.contains("tensor") ||
                                 content.contains("inference") ||
                                 content.contains("graph-execution-context");
    
    // Matrix workload detection based on patterns
    memory_info.is_matrix_workload = !memory_info.is_ml_workload && 
                                     (content.contains("matrix") ||
                                      content.contains("transpose") ||
                                      content.contains("multiplication") ||
                                      memory_info.function_count > 50);
    
    // Simple workload detection
    memory_info.is_simple_workload = !memory_info.is_ml_workload && 
                                     !memory_info.is_matrix_workload &&
                                     memory_info.function_count <= 50 &&
                                     memory_info.import_count <= 10;
    
    // === COMPLEXITY SCORE CALCULATION ===
    // Calculate a complexity score (0-100) based on multiple factors
    let mut complexity_score = 0u32;
    
    // Binary size factor (0-20 points)
    if memory_info.binary_size_bytes > 1_000_000 {
        complexity_score += 20;
    } else if memory_info.binary_size_bytes > 500_000 {
        complexity_score += 15;
    } else if memory_info.binary_size_bytes > 100_000 {
        complexity_score += 10;
    } else {
        complexity_score += 5;
    }
    
    // Import/export complexity (0-15 points)
    complexity_score += (memory_info.import_count + memory_info.export_count).min(15);
    
    // Data section complexity (0-15 points)
    complexity_score += (memory_info.data_section_count * 2).min(15);
    
    // Function complexity (0-20 points)
    complexity_score += (memory_info.function_count / 10).min(20);
    
    // Type/instance complexity (0-15 points)
    complexity_score += ((memory_info.type_definition_count + memory_info.instance_count) / 2).min(15);
    
    // Local variable complexity (0-15 points)
    complexity_score += (memory_info.high_complexity_functions * 3).min(15);
    
    memory_info.complexity_score = complexity_score.min(100);
    
    // Print analysis results
    println!("📊 Advanced WAT Analysis:");
    println!("   • Linear memory: {} pages ({:.2} MB)", 
             memory_info.linear_memory_pages, 
             memory_info.linear_memory_bytes as f64 / (1024.0 * 1024.0));
    println!("   • Stack offset: {:.2} MB", 
             memory_info.stack_pointer_offset as f64 / (1024.0 * 1024.0));
    println!("   • Imports: {}, Exports: {}", memory_info.import_count, memory_info.export_count);
    println!("   • Data sections: {} ({:.2} MB)", 
             memory_info.data_section_count,
             memory_info.data_section_size_bytes as f64 / (1024.0 * 1024.0));
    println!("   • Functions: {}, Max locals: {}, High complexity: {}", 
             memory_info.function_count,
             memory_info.max_local_variables_per_function,
             memory_info.high_complexity_functions);
    println!("   • Types: {}, Instances: {}, Resources: {}", 
             memory_info.type_definition_count,
             memory_info.instance_count,
             memory_info.resource_count);
    println!("   • Complexity score: {}/100", memory_info.complexity_score);
    
    let workload_type = if memory_info.is_ml_workload {
        "ML Inference"
    } else if memory_info.is_matrix_workload {
        "Matrix Operations"
    } else if memory_info.is_simple_workload {
        "Simple Computation"
    } else {
        "Complex Computation"
    };
    println!("   • Workload type: {}", workload_type);
    
    Ok(())
}

/// Analyze request payload size
fn analyze_request_payload_size(payload: &str, memory_info: &mut MemoryInfoEstimatorAdvanced) {
    memory_info.request_payload_size = payload.len() as u64;
}

/// Analyze model file size for ML tasks
fn analyze_model_file_size(model_folder_name: &str, memory_info: &mut MemoryInfoEstimatorAdvanced) -> Result<(), std::io::Error> {
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

/// Calculate peak memory requirement using binary size as base + WAT field adjustments
/// 
/// This estimator uses a more logical approach:
/// - Base memory: Binary size (the actual WASM module needs to be loaded)
/// - WAT field adjustments: Additional memory based on component complexity
/// - Workload-specific multipliers: Different overhead for different task types
pub fn calculate_peak_memory_advanced(memory_info: &mut MemoryInfoEstimatorAdvanced) -> u64 {
    // Base memory = binary size (the WASM module itself needs to be loaded)
    let base_memory = memory_info.binary_size_bytes;
    
    // Data section overhead (static data needs additional memory)
    let data_overhead = memory_info.data_section_size_bytes;
    
    // WAT field-based memory adjustments
    let wat_adjustments = 
        // Import overhead (external dependencies)
        (memory_info.import_count as u64 * 1024) +  // 1KB per import
        // Export overhead (interface complexity)  
        (memory_info.export_count as u64 * 512) +  // 512B per export
        // Function overhead (code complexity)
        (memory_info.function_count as u64 * 2048) +  // 2KB per function
        // Global variable overhead (state management)
        (memory_info.global_variable_count as u64 * 256) +  // 256B per global
        // Type definition overhead (interface complexity)
        (memory_info.type_definition_count as u64 * 128) +  // 128B per type
        // Instance overhead (component complexity)
        (memory_info.instance_count as u64 * 1024) +  // 1KB per instance
        // Resource overhead (memory management)
        (memory_info.resource_count as u64 * 512) +  // 512B per resource
        // Local variable overhead (function complexity)
        (memory_info.total_local_variables as u64 * 64);  // 64B per local variable
    
    // Workload-specific multipliers based on complexity score
    let workload_multiplier = if memory_info.is_ml_workload {
        // ML tasks: Higher multiplier due to ONNX runtime overhead
        // Base multiplier + complexity factor
        1.5 + (memory_info.complexity_score as f64 / 100.0) * 0.5  // 1.5x to 2.0x
    } else if memory_info.is_matrix_workload {
        // Matrix tasks: Medium multiplier for computation buffers
        1.2 + (memory_info.complexity_score as f64 / 100.0) * 0.3  // 1.2x to 1.5x
    } else if memory_info.is_simple_workload {
        // Simple tasks: Low multiplier
        1.0 + (memory_info.complexity_score as f64 / 100.0) * 0.2  // 1.0x to 1.2x
    } else {
        // Complex tasks: High multiplier
        1.3 + (memory_info.complexity_score as f64 / 100.0) * 0.4  // 1.3x to 1.7x
    };
    
    // Calculate total estimated memory
    let raw_estimate = base_memory + data_overhead + wat_adjustments;
    memory_info.estimated_minimum_memory_bytes = raw_estimate;
    memory_info.estimated_peak_memory_bytes = (raw_estimate as f64 * workload_multiplier) as u64;
    
    // Print detailed memory breakdown
    println!("🧮 Advanced Memory Calculation (Binary-Based):");
    println!("   • Base memory (binary size): {:.2} MB", base_memory as f64 / (1024.0 * 1024.0));
    println!("   • Data overhead: {:.2} MB", data_overhead as f64 / (1024.0 * 1024.0));
    println!("   • WAT field adjustments: {:.2} MB", wat_adjustments as f64 / (1024.0 * 1024.0));
    println!("   • Raw estimate: {:.2} MB", raw_estimate as f64 / (1024.0 * 1024.0));
    println!("   • Workload multiplier: {:.2}x", workload_multiplier);
    println!("   • Total estimated: {:.2} MB", memory_info.estimated_peak_memory_bytes as f64 / (1024.0 * 1024.0));
    
    memory_info.estimated_peak_memory_bytes
}

/// Build comprehensive memory information using advanced estimator
pub fn build_memory_info_advanced(wasm_file: &str, wat_file: &str, payload: &str, model_folder_name: &str) -> MemoryInfoEstimatorAdvanced {
    let mut memory_info = MemoryInfoEstimatorAdvanced::new();
    
    // 1. Analyze WASM binary file size
    match analyze_binary_size(wasm_file, &mut memory_info) {
        Ok(_) => {
            println!("📦 Binary Analysis:");
            println!("   • File size: {:.2} MB", memory_info.binary_size_mb);
        },
        Err(e) => println!("Error analyzing binary: {}", e),
    }
    
    // 2. Advanced WAT analysis
    match analyze_wat_advanced(wat_file, &mut memory_info) {
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
    
    // 5. Calculate final memory estimates using advanced algorithm
    let _peak_memory_estimated = calculate_peak_memory_advanced(&mut memory_info);

    memory_info
}

/// Print advanced memory analysis summary
pub fn print_memory_analysis_advanced(memory_info: &MemoryInfoEstimatorAdvanced) {
    println!("\n🎯 Advanced Memory Estimation Summary:");
    println!("   • Binary Size: {:.2} MB", memory_info.binary_size_mb);
    println!("   • Payload Size: {:.2} MB", memory_info.request_payload_size as f64 / (1024.0 * 1024.0));
    println!("   • Complexity Score: {}/100", memory_info.complexity_score);
    
    let workload_type = if memory_info.is_ml_workload {
        "ML Inference"
    } else if memory_info.is_matrix_workload {
        "Matrix Operations"
    } else if memory_info.is_simple_workload {
        "Simple Computation"
    } else {
        "Complex Computation"
    };
    println!("   • Workload Type: {}", workload_type);
    println!("   • Estimated Peak Memory: {:.2} MB", 
             memory_info.estimated_peak_memory_bytes as f64 / (1024.0 * 1024.0));
    
    if memory_info.is_ml_workload {
        println!("   • ML task detected - complexity-based overhead for ONNX runtime");
    } else if memory_info.is_matrix_workload {
        println!("   • Matrix task detected - complexity-based overhead for computation buffers");
    } else if memory_info.is_simple_workload {
        println!("   • Simple task detected - minimal overhead for basic runtime");
    } else {
        println!("   • Complex task detected - high overhead for advanced runtime");
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
