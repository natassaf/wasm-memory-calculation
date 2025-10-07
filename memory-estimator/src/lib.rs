use std::fs;
use wasmprinter;
use regex::Regex;
use std::fmt;

#[derive(Debug, Clone)]
pub struct MemoryInfo {
    pub linear_memory_pages: u32,
    pub linear_memory_bytes: u64,
    pub stack_pointer_offset: u64,
    pub function_tables: Vec<u32>,
    pub total_function_references: u32,
    pub estimated_minimum_memory_bytes: u64,
    pub estimated_peak_memory_bytes: u64,
    pub is_ml_workload: bool,
    pub binary_size_bytes: u64,
    pub binary_size_mb: f64,
}
impl MemoryInfo {

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
            binary_size_bytes: 0,
            binary_size_mb: 0.0,
        }
    }
}


impl fmt::Display for MemoryInfo {
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
pub fn analyze_binary_size(wasm_path: &str, memory_info: &mut MemoryInfo) -> Result<(), Box<dyn std::error::Error>> {
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

/// Analyze memory requirements from a .wat file (SIMPLIFIED)
pub fn analyze_wat_memory_simple(wat_path: &str, memory_info: &mut MemoryInfo) -> Result<(), Box<dyn std::error::Error>> {
    let content = fs::read_to_string(wat_path)?;
    
    // Simplified regex patterns for key metrics
    let memory_regex = Regex::new(r"\(memory.*?(\d+)\)")?;
    let stack_regex = Regex::new(r"stack_pointer.*?(\d+)")?;
    let table_regex = Regex::new(r"\(table.*?(\d+).*?(\d+).*?funcref\)")?;
    
    // Find memory pages
    if let Some(caps) = memory_regex.captures(&content) {
        memory_info.linear_memory_pages = caps[1].parse()?;
        memory_info.linear_memory_bytes = memory_info.linear_memory_pages as u64 * 65536;
    }
    
    // Find stack pointer
    if let Some(caps) = stack_regex.captures(&content) {
        memory_info.stack_pointer_offset = caps[1].parse()?;
    }
    
    // Find function tables
    for caps in table_regex.captures_iter(&content) {
        let table_size = caps[1].parse::<u32>()?;
        memory_info.function_tables.push(table_size);
    }
    
    memory_info.total_function_references = memory_info.function_tables.iter().sum();
    
    Ok(())
}

pub fn calculate_aggregated_memory(memory_info: &mut MemoryInfo) -> () {
    // Simple ML detection based on binary size and function count
    memory_info.is_ml_workload = memory_info.binary_size_bytes > 200_000 || 
    memory_info.total_function_references > 150;

    // Calculate estimates 
    memory_info.estimated_minimum_memory_bytes = 
memory_info.binary_size_bytes +           // Static binary footprint
    memory_info.linear_memory_bytes +         // Data memory
    memory_info.stack_pointer_offset;         // Stack space

    let buffer_size = if memory_info.is_ml_workload {
    5 * 1024 * 1024 // 5MB buffer for ML workloads
    } else {
    1024 * 1024 // 1MB buffer for regular workloads
    };

    memory_info.estimated_peak_memory_bytes = memory_info.estimated_minimum_memory_bytes + buffer_size;
}

/// Print simplified memory analysis
pub fn print_memory_analysis_simple(memory_info: &MemoryInfo) {
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
