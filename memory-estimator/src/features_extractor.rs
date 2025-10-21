use regex::Regex;
use std::fs;
use std::io::Write;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct MemoryFeatures {
    // Size analysis
    pub binary_size_bytes: u64,
    pub data_section_size_bytes: u64,

    // Advanced WAT counts
    pub import_count: u32,
    pub export_count: u32,
    pub function_count: u32,
    pub global_variable_count: u32,
    pub type_definition_count: u32,
    pub instance_count: u32,
    pub resource_count: u32,

    // Function locals complexity
    pub total_local_variables: u32,
    pub max_local_variables_per_function: u32,
    pub avg_local_variables_per_function: f32,
    pub high_complexity_functions: u32,

    // Core memory components
    pub linear_memory_bytes: u64,
    pub stack_pointer_offset: u64,
    pub total_function_references: u32,

    // Classification and request sizes
    pub is_ml_workload: bool,
    pub request_payload_size: u64,
    pub model_file_size: u64,
    pub memory_kb: i64
}

impl MemoryFeatures {
    pub fn csv_header() -> &'static str {
        "binary_size_bytes,data_section_size_bytes,import_count,export_count,function_count,global_variable_count,type_definition_count,instance_count,resource_count,total_local_variables,max_local_variables_per_function,avg_local_variables_per_function,high_complexity_functions,linear_memory_bytes,stack_pointer_offset,total_function_references,is_ml_workload,request_payload_size,model_file_size, memory_kb"
    }

    pub fn to_csv_row(&self) -> String {
        format!(
            "{},{},{},{},{},{},{},{},{},{},{:.4},{},{},{},{},{},{},{},{}",
            self.binary_size_bytes,
            self.data_section_size_bytes,
            self.import_count,
            self.export_count,
            self.function_count,
            self.global_variable_count,
            self.type_definition_count,
            self.instance_count,
            self.resource_count,
            self.total_local_variables,
            self.avg_local_variables_per_function,
            self.high_complexity_functions,
            self.linear_memory_bytes,
            self.stack_pointer_offset,
            self.total_function_references,
            if self.is_ml_workload { 1 } else { 0 },
            self.request_payload_size,
            self.model_file_size,
            self.memory_kb,
        )
    }
}

pub fn extract_features(
    wasm_file: &str,
    wat_file: &str,
    payload: &str,
    model_folder_name: &str,
    memory_kb: Option<u64>,
) -> MemoryFeatures {
    // Binary size
    let binary_size_bytes = fs::metadata(wasm_file).map(|m| m.len()).unwrap_or(0);

    // Read WAT
    let content = fs::read_to_string(wat_file).unwrap_or_else(|_| String::new());

    // Linear memory
    let mut linear_memory_bytes: u64 = 0;
    let memory_regex = Regex::new(r"\(memory\s+\(;\d+;\)\s+(\d+)\)").unwrap_or_else(|_| Regex::new("$").unwrap());
    if let Some(c) = memory_regex.captures(&content) {
        if let Ok(pages) = c[1].parse::<u32>() {
            linear_memory_bytes = pages as u64 * 64 * 1024;
        }
    }

    // Stack pointer
    let mut stack_pointer_offset: u64 = 0;
    let stack_regex = Regex::new(r"stack_pointer\s+i32\.const\s+(\d+)").unwrap_or_else(|_| Regex::new("$").unwrap());
    if let Some(c) = stack_regex.captures(&content) {
        if let Ok(v) = c[1].parse::<u64>() { stack_pointer_offset = v; }
    }

    // Function tables → total refs
    let mut total_function_references: u32 = 0;
    let table_regex = Regex::new(r"\(table\s+\d+\s+(\d+)\s+funcref\)").unwrap_or_else(|_| Regex::new("$").unwrap());
    for cap in table_regex.captures_iter(&content) {
        if let Ok(size) = cap[1].parse::<u32>() { total_function_references += size; }
    }

    // Counts
    let import_count = content.matches("(import ").count() as u32;
    let export_count = content.matches("(export ").count() as u32;
    let function_count = content.matches("(func ").count() as u32;
    let global_variable_count = content.matches("(global ").count() as u32;
    let type_definition_count = content.matches("(type ").count() as u32;
    let instance_count = content.matches("(instance ").count() as u32;
    let resource_count = content.matches("(resource ").count() as u32;

    // Data section size (rough estimate by byte length of (data ...) blocks)
    let mut data_section_size_bytes: u64 = 0;
    let data_size_regex = Regex::new(r"\(data[\s\S]*?\)").unwrap_or_else(|_| Regex::new("$").unwrap());
    for m in data_size_regex.find_iter(&content) {
        data_section_size_bytes += (m.end() - m.start()) as u64;
    }

    // Local variables analysis
    let local_regex = Regex::new(r"\(local\s+([^)]*)\)").unwrap_or_else(|_| Regex::new("$").unwrap());
    let mut total_local_variables: u32 = 0;
    let mut max_local_variables_per_function: u32 = 0;
    let mut high_complexity_functions: u32 = 0;
    let mut locals_entries: u32 = 0;
    for caps in local_regex.captures_iter(&content) {
        let text = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let count = text.split_whitespace().filter(|s| !s.is_empty()).count() as u32;
        total_local_variables += count;
        if count > max_local_variables_per_function { max_local_variables_per_function = count; }
        if count > 10 { high_complexity_functions += 1; }
        locals_entries += 1;
    }
    let avg_local_variables_per_function: f32 = if locals_entries > 0 {
        total_local_variables as f32 / locals_entries as f32
    } else { 0.0 };

    // ML workload detection (WASI-NN indicators)
    let lower = content.to_lowercase();
    let is_ml_workload = [
        "wasi:nn", "wasi-nn", "tensor", "inference", "graph-execution-context",
    ].iter().any(|k| lower.contains(k));

    // Request payload and model size
    let request_payload_size = payload.len() as u64;
    let model_file_size = compute_model_folder_size(model_folder_name);

    MemoryFeatures {
        binary_size_bytes,
        data_section_size_bytes,
        import_count,
        export_count,
        function_count,
        global_variable_count,
        type_definition_count,
        instance_count,
        resource_count,
        total_local_variables,
        max_local_variables_per_function,
        avg_local_variables_per_function,
        high_complexity_functions,
        linear_memory_bytes,
        stack_pointer_offset,
        total_function_references,
        is_ml_workload,
        request_payload_size,
        model_file_size,
        memory_kb: memory_kb.map(|kb| kb as i64).unwrap_or(-1),
    }
}

fn compute_model_folder_size(model_folder_name: &str) -> u64 {
    if model_folder_name.is_empty() { return 0; }
    let base = if cfg!(target_os = "linux") {
        "/home/pi/memory-estimator/models/"
    } else {
        "/Users/athanasiapharmake/workspace/wasm-memory-calculation/memory-estimator/models/"
    };
    let folder = format!("{}{}/", base, model_folder_name);
    if let Ok(entries) = fs::read_dir(&folder) {
        let mut total = 0u64;
        for e in entries.flatten() {
            if let Ok(meta) = e.metadata() { if meta.is_file() { total += meta.len(); } }
        }
        total
    } else { 0 }
}

pub fn append_features_to_csv(csv_path: &str, features: &MemoryFeatures) -> std::io::Result<()> {
    let path = Path::new(csv_path);
    let mut file = if path.exists() {
        fs::OpenOptions::new().append(true).open(path)?
    } else {
        let mut f = fs::OpenOptions::new().create(true).write(true).open(path)?;
        writeln!(f, "{}", MemoryFeatures::csv_header())?;
        f
    };
    writeln!(file, "{}", features.to_csv_row())?;
    Ok(())
}


