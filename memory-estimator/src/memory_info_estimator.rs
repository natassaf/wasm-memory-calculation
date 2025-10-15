use std::fs;
use wasmprinter;
use regex::Regex;
use std::fmt;

#[derive(Debug, Clone)]
pub struct MemoryInfoEstimator {
    pub linear_memory_pages: u32,
    pub linear_memory_bytes: u64,
    pub stack_pointer_offset: u64,
    pub function_tables: Vec<u32>,
    pub total_function_references: u32,
    pub estimated_minimum_memory_bytes: u64,
    pub estimated_peak_memory_bytes: u64,
    pub is_ml_workload: bool,
    pub is_matrix_workload: bool,
    pub is_simple_workload: bool,
    pub binary_size_bytes: u64,
    pub binary_size_mb: f64,
}
impl MemoryInfoEstimator {

    pub fn new() -> Self {
        Self {
            linear_memory_pages: 0,
            linear_memory_bytes: 0,
            stack_pointer_offset: 0,
            function_tables: Vec::new(),
            total_function_references: 0,
            estimated_minimum_memory_bytes: 0,
            estimated_peak_memory_bytes: 0,
            is_ml_workload: false,
            is_matrix_workload: false,
            is_simple_workload: false,
            binary_size_bytes: 0,
            binary_size_mb: 0.0,
        }
    }
    
}


impl fmt::Display for MemoryInfoEstimator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MemoryInfo:\n\
             - linear_memory_pages: {}\n\
             - linear_memory_bytes: {}\n\
             - stack_pointer_offset: {}\n\
             - function_tables: {:?}\n\
             - total_function_references: {}\n\
             - estimated_minimum_memory: {}\n\
             - estimated_peak_memory: {}\n\
             - is_ml_workload: {}\n\
             - binary_size_bytes: {}\n\
             - binary_size_mb: {:.2}",
            self.linear_memory_pages,
            self.linear_memory_bytes,
            self.stack_pointer_offset,
            self.function_tables,
            self.total_function_references,
            self.estimated_minimum_memory_bytes,
            self.estimated_peak_memory_bytes,
            self.is_ml_workload,
            self.binary_size_bytes,
            self.binary_size_mb,
        )
    }
}


/// Analyze binary size of WASM file
pub fn analyze_binary_size(wasm_path: &str, memory_info: &mut MemoryInfoEstimator) -> Result<(), Box<dyn std::error::Error>> {
    let metadata = fs::metadata(wasm_path)?;
    let binary_size_bytes = metadata.len();
    let binary_size_mb = binary_size_bytes as f64 / (1024.0 * 1024.0);
    memory_info.binary_size_bytes = binary_size_bytes;
    memory_info.binary_size_mb = binary_size_mb;
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

/// Analyze memory requirements from a .wat file (ENHANCED)
pub fn analyze_wat_memory_simple(wat_path: &str, memory_info: &mut MemoryInfoEstimator) -> Result<(), Box<dyn std::error::Error>> {
    let content = fs::read_to_string(wat_path)?;
    
    // Enhanced regex patterns for comprehensive analysis
    let memory_regex = Regex::new(r"\(memory.*?(\d+)\)")?;
    let stack_regex = Regex::new(r"stack_pointer.*?i32\.const\s+(\d+)")?;
    let table_regex = Regex::new(r"\(table.*?(\d+).*?(\d+).*?funcref\)")?;
    let func_regex = Regex::new(r"^\s*\(func ")?;
    let data_regex = Regex::new(r"^\s*\(data ")?;
    let global_regex = Regex::new(r"^\s*\(global ")?;
    
    // Find memory pages
    if let Some(caps) = memory_regex.captures(&content) {
        memory_info.linear_memory_pages = caps[1].parse()?;
        memory_info.linear_memory_bytes = memory_info.linear_memory_pages as u64 * 65536;
    }
    
    // Find stack pointer offset (more precise pattern)
    if let Some(caps) = stack_regex.captures(&content) {
        memory_info.stack_pointer_offset = caps[1].parse()?;
    }
    
    // Find function tables
    for caps in table_regex.captures_iter(&content) {
        let table_size = caps[1].parse::<u32>()?;
        memory_info.function_tables.push(table_size);
    }
    
    memory_info.total_function_references = memory_info.function_tables.iter().sum();
    
    // Count functions for complexity analysis (simpler approach)
    let function_count = content.lines()
        .filter(|line| line.trim().starts_with("(func "))
        .count();
    
    // Count data sections for static data analysis
    let data_section_count = content.lines()
        .filter(|line| line.trim().starts_with("(data "))
        .count();
    
    // Count globals for state analysis
    let global_count = content.lines()
        .filter(|line| line.trim().starts_with("(global "))
        .count();
    
    // Enhanced workload classification based on multiple factors
    // ML inference workloads are specifically neural network models (ONNX, etc.)
    // These typically have moderate size but high function count and data sections
    let is_ml_inference = memory_info.binary_size_bytes > 600_000 || 
                         (function_count > 500 && data_section_count > 2) ||
                         (memory_info.binary_size_bytes > 300_000 && function_count > 400) ||
                         (memory_info.binary_size_bytes > 200_000 && function_count > 500);
    
    // Matrix operations are computational workloads with moderate complexity
    let is_matrix_workload = memory_info.binary_size_bytes > 150_000 && 
                            memory_info.binary_size_bytes <= 600_000 &&
                            function_count > 200 && function_count <= 500 &&
                            !is_ml_inference;
    
    // Simple computational workloads (Fibonacci, basic algorithms)
    let is_simple_workload = memory_info.binary_size_bytes <= 150_000 || 
                            function_count <= 200;
    
    // Update workload classification flags
    memory_info.is_ml_workload = is_ml_inference;
    memory_info.is_matrix_workload = is_matrix_workload;
    memory_info.is_simple_workload = is_simple_workload;
    
    println!("📊 Enhanced WAT Analysis:");
    println!("   • Functions: {}", function_count);
    println!("   • Data sections: {}", data_section_count);
    println!("   • Globals: {}", global_count);
    println!("   • Stack pointer: {} bytes ({:.2} MB)", 
             memory_info.stack_pointer_offset,
             memory_info.stack_pointer_offset as f64 / (1024.0 * 1024.0));
    
    // Print workload classification
    let workload_type = if memory_info.is_ml_workload {
        "ML Inference"
    } else if memory_info.is_matrix_workload {
        "Matrix Operations"
    } else if memory_info.is_simple_workload {
        "Simple Computation"
    } else {
        "Unclassified"
    };
    println!("   • Workload type: {}", workload_type);
    
    Ok(())
}

pub fn calculate_aggregated_memory(memory_info: &mut MemoryInfoEstimator) -> () {
    // Enhanced memory calculation using stack pointer and linear memory
    // Base memory = linear memory + stack space
    let base_memory = memory_info.linear_memory_bytes + memory_info.stack_pointer_offset;
    
    // Add binary overhead (typically 10-20% of binary size for runtime overhead)
    let binary_overhead = (memory_info.binary_size_bytes as f64 * 0.15) as u64;
    
    // Calculate minimum memory requirement
    memory_info.estimated_minimum_memory_bytes = base_memory + binary_overhead;

    // Dynamic buffer calculation based on workload complexity
    let buffer_size = if memory_info.is_ml_workload {
        // ML inference workloads (SqueezeNet, ResNet) - large buffer for model operations
        if memory_info.binary_size_bytes > 500_000 {
            15 * 1024 * 1024 // 15MB for large ML models
        } else {
            12 * 1024 * 1024 // 12MB for medium ML models
        }
    } else if memory_info.is_matrix_workload {
        // Matrix operations (multiplication, transpose) - medium buffer
        if memory_info.binary_size_bytes > 200_000 {
            8 * 1024 * 1024  // 8MB for large matrix operations
        } else {
            6 * 1024 * 1024  // 6MB for medium matrix operations
        }
    } else if memory_info.is_simple_workload {
        // Simple computational workloads (Fibonacci) - small buffer
        if memory_info.total_function_references > 50 {
            3 * 1024 * 1024  // 3MB for complex simple workloads
        } else {
            2 * 1024 * 1024  // 2MB for basic simple workloads
        }
    } else {
        // Fallback for unclassified workloads
        5 * 1024 * 1024      // 5MB default buffer
    };

    memory_info.estimated_peak_memory_bytes = memory_info.estimated_minimum_memory_bytes + buffer_size;
    
    println!("🧮 Memory Calculation:");
    println!("   • Base memory: {:.2} MB (linear: {:.2} MB + stack: {:.2} MB)", 
             base_memory as f64 / (1024.0 * 1024.0),
             memory_info.linear_memory_bytes as f64 / (1024.0 * 1024.0),
             memory_info.stack_pointer_offset as f64 / (1024.0 * 1024.0));
    println!("   • Binary overhead: {:.2} MB", binary_overhead as f64 / (1024.0 * 1024.0));
    println!("   • Buffer size: {:.2} MB", buffer_size as f64 / (1024.0 * 1024.0));
}


pub fn build_memory_info(wasm_file: &str, wat_file: &str) -> MemoryInfoEstimator {
    let mut memory_info = MemoryInfoEstimator::new();
    
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

/// Print simplified memory analysis
pub fn print_memory_analysis_simple(memory_info: &MemoryInfoEstimator) {
    println!("📊 Linear Memory:");
    println!("   • Pages: {} (64KB each)", memory_info.linear_memory_pages);
    println!("   • Total: {:.2} MB", memory_info.linear_memory_bytes as f64 / (1024.0 * 1024.0));
    
    println!("\n📚 Stack Memory:");
    println!("   • Stack pointer offset: {:.2} MB", memory_info.stack_pointer_offset as f64 / (1024.0 * 1024.0));
    
    println!("\n🔗 Function Tables:");
    for (i, table_size) in memory_info.function_tables.iter().enumerate() {
        println!("   • Table {}: {} function references", i, table_size);
    }
    println!("   • Total function references: {}", memory_info.total_function_references);
    
    // Workload analysis
    if memory_info.is_ml_workload {
        println!("\n🤖 ML Workload Analysis:");
        println!("   • Workload Type: Machine Learning / AI Inference");
        println!("   • High function complexity detected");
    } else {
        println!("\n⚙️ Standard Workload Analysis:");
        println!("   • Workload Type: General Purpose");
    }
    
    println!("\n💾 Memory Summary:");
    println!("   • Minimum memory: {:.2} MB", memory_info.estimated_minimum_memory_bytes as f64 / (1024.0 * 1024.0));
    println!("   • Estimated peak: {:.2} MB", memory_info.estimated_peak_memory_bytes as f64 / (1024.0 * 1024.0));
    
    println!("\n🎯 Recommendations:");
    if memory_info.is_ml_workload {
        println!("   • ML workload detected - allocate extra memory for model operations");
    } else {
        if memory_info.linear_memory_pages < 32 {
            println!("   • Memory usage is efficient (under 2MB)");
        } else {
            println!("   • Consider optimizing memory usage");
        }
    }
    
    println!("   • Allocate at least {:.2} MB for safe execution",
             memory_info.estimated_peak_memory_bytes as f64 / (1024.0 * 1024.0));
}
