
use std::{env, fs};
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use memory_estimator::memory_info_estimator::{build_memory_info, convert_wasm_to_wat, print_memory_analysis_simple, MemoryInfoEstimator, detect_ml_task_from_wat};
use memory_estimator::memory_info_estimator_advanced::{build_memory_info_advanced, MemoryInfoEstimatorAdvanced};
use memory_estimator::memory_info_estimator_conservative::{build_memory_info_conservative, MemoryInfoEstimatorConservative};
use memory_estimator::memory_info_estimator_improved::{build_memory_info_improved, MemoryInfoEstimatorImproved};
use memory_estimator::features_extractor::{extract_features, append_features_to_csv};
use memory_estimator::various::append_data_to_file;
use memory_estimator::wasm_loader_basic::run_wasm_job_component_basic;
use memory_estimator::wasm_loader_wasi_nn::run_wasm_job_component_with_wasi_nn;
use serde::{Deserialize, Serialize};
use serde_json;
use base64::{Engine as _, engine::general_purpose};
use actix_web::{web, App, HttpResponse, HttpServer, Responder};

static WASM_MODULES_FOLDER: &str = if cfg!(target_os = "linux") {
    "/home/pi/memory-estimator/wasm-modules/"
} else {
    "/Users/athanasiapharmake/workspace/wasm-memory-calculation/memory-estimator/wasm-modules/"
};

static WASM_MODELS_FOLDER: &str = if cfg!(target_os = "linux") {
    "/home/pi/memory-estimator/models/"
} else {
    "/Users/athanasiapharmake/workspace/wasm-memory-calculation/memory-estimator/models/"
};

static RESULTS_FOLDER: &str = if cfg!(target_os = "linux") {
    "/home/pi/memory-estimator/results/"
} else {
    "/Users/athanasiapharmake/workspace/wasm-memory-calculation/memory-estimator/results/"
};

fn get_peak_memory_usage() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        let pid = std::process::id() as usize;
        // let status_content = std::fs::read_to_string(format!("/proc/{}/status", pid));
        // println!("Status content: {:?}", status_content);
        // let status_content_2 = std::fs::read_to_string(format!("/proc/{}/statm", pid));
        // println!("Status content 2: {:?}", status_content_2);

        let content = fs::read_to_string(format!("/proc/{}/status", pid)).ok()?;
        for line in content.lines() {
            if line.starts_with("VmHWM:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    return parts[1].parse::<u64>().ok();
                }
            }
        }
       return  None;
    }
    
    None
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WasmJobRequest{
    binary_name: String,
    func_name: String,
    payload: String,
    payload_compressed: bool,
    task_id: String,
    model_folder_name: String,
    cwasm_file: String,
    wat_file: String,
}

fn spawn_child_process(task: WasmJobRequest) {
    let current_pid = std::process::id() as usize;
    println!("Parent pid {}: spawning child process for task {}", current_pid, task.task_id);

    let task_file = format!("/tmp/wasm_task_{}.json", task.task_id);
    let task_json = serde_json::to_string(&task).unwrap();
    std::fs::write(&task_file, task_json).expect("Failed to write task");
    let current_exe = env::current_exe().unwrap();
    println!("Current exe: {:?}", current_exe);
    let mut child = Command::new(current_exe)
        .arg("child")
        .arg(&task_file) // Only pass the task file path
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn child");

    let output = child.wait_with_output().expect("Failed to wait for child");
    
    // Clean up temp file
    let _ = std::fs::remove_file(&task_file);
    
    if output.status.success() {
        println!("Child output: {}", String::from_utf8_lossy(&output.stdout));
    } else {
        println!("Child error: {}", String::from_utf8_lossy(&output.stderr));
        println!("Child stdout: {}", String::from_utf8_lossy(&output.stdout));
    }
}

async fn run_child(task: WasmJobRequest)->Option<u64> {
    println!("Child: running wasm job component...");
    

    // Handle compressed payload
    let payload = if task.payload_compressed {
        // Decompress the payload
        let compressed_bytes = general_purpose::STANDARD.decode(&task.payload).expect("Failed to decode base64");
        let mut decoder = flate2::read::GzDecoder::new(&compressed_bytes[..]);
        let mut decompressed = String::new();
        std::io::Read::read_to_string(&mut decoder, &mut decompressed).expect("Failed to decompress");
        decompressed
    } else {
        // Payload is already uncompressed
        task.payload
    };
    let component_path = WASM_MODULES_FOLDER.to_string() + &task.binary_name;
    let wat_file_path = WASM_MODULES_FOLDER.to_string() + &task.wat_file;
    println!("Component path: {}", component_path);
    
    // Get memory before WASM execution
    let memory_before_wasm = get_peak_memory_usage();
    if let Some(m) = memory_before_wasm {
        println!("Child: Memory before WASM execution: {} mb", m / 1024);
    }
    match detect_ml_task_from_wat(&wat_file_path) {
        Ok(is_ml_task) => {
            if is_ml_task {
                let folder_to_mount: String = WASM_MODELS_FOLDER.to_string();
                println!("Child: is_ml_task: {}", is_ml_task);
                match run_wasm_job_component_with_wasi_nn(
                    task.task_id,
                    component_path,
                    task.func_name,
                    payload,
                    folder_to_mount,
                ).await {
                    Ok(result) => println!("Child result: {:?}", result),
                    Err(e) => println!("Child error: {:?}", e),
                }
            
            } else {
                match run_wasm_job_component_basic(
                    task.task_id,
                    component_path,
                    task.func_name,
                    payload,
                    task.model_folder_name,
                ).await {
                    Ok(result) => println!("Child result: {:?}", result),
                    Err(e) => println!("Child error: {:?}", e),
                }
            }
    },
        Err(e) => println!("Child error: {:?}", e),
    }
    let mut peak_memory_monitored_kb: Option<u64>= get_peak_memory_usage();

    match (peak_memory_monitored_kb, memory_before_wasm){
        (Some(memory_after), Some(memory_before)) => {
            println!("Child: Memory after WASM execution: {} MB", memory_after / 1024);        }
        (_, _) => {
            println!("Child: Memory after WASM execution: Not available");
        }
    }

    return peak_memory_monitored_kb;
}

async fn run_task(task: WasmJobRequest){
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 && args[1] == "child" {
        run_child(task).await;
    } else {
        spawn_child_process(task);
    }

}
async fn handle_submit_task(task: web::Json<WasmJobRequest>)->impl Responder{
    run_task(task.into_inner()).await;
    HttpResponse::Ok().body("Task done")
}

async fn handle_plot_memory()->impl Responder{
    HttpResponse::Ok().body("Plots ok")
}

#[actix_web::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    // If this is a child process, run the WASM task and exit
    if args.len() > 1 && args[1] == "child" {
        // Parse command line arguments for child process
        if args.len() >= 3 {
            let task_file = &args[2];
            let task_json = std::fs::read_to_string(task_file).expect("Failed to read task file");
            let task: WasmJobRequest = serde_json::from_str(&task_json).expect("Failed to parse task JSON");
            
            // Construct full file paths
            let cwasm_file: String = WASM_MODULES_FOLDER.to_string() + &task.cwasm_file;
            let wat_file: String = WASM_MODULES_FOLDER.to_string() + &task.wat_file;
            let wasm_file: String =WASM_MODELS_FOLDER.to_string() + &task.binary_name;
            
            // Convert WASM to WAT only if .wat file doesn't exist
            if !std::path::Path::new(&wat_file).exists() {
                match convert_wasm_to_wat(&wasm_file, &wat_file) {
                    Ok(_) => println!("Successfully converted {} to {}", cwasm_file, wat_file),
                    Err(e) => println!("Error converting file: {}", e),
                }
            }

            // Use payload content directly for memory analysis (it's already the content, not a file path)
            let payload = if task.payload_compressed {
                // Decompress the payload for analysis
                let compressed_bytes = base64::engine::general_purpose::STANDARD.decode(&task.payload).expect("Failed to decode base64");
                let mut decoder = flate2::read::GzDecoder::new(&compressed_bytes[..]);
                let mut decompressed = String::new();
                std::io::Read::read_to_string(&mut decoder, &mut decompressed).expect("Failed to decompress");
                decompressed
            } else {
                task.payload.clone()
            };

            let memory_info: MemoryInfoEstimatorConservative = build_memory_info_conservative(&cwasm_file, &wat_file, &payload, &task.model_folder_name);

            // println!("Estimated memory info: {}", memory_info);
            // print_memory_analysis_simple(&memory_info);

            let peak_memory_monitored_kb = run_child(task.clone()).await;
            
            if let Some(peak_memory_monitored) = peak_memory_monitored_kb {
                let peak_memory_monitored = peak_memory_monitored as f64 /  1024.0;
                println!("Peak memory monitored: {} MB", peak_memory_monitored);
            } else {
                println!("Peak memory monitored: Not available");
            }

            // Extract features and store row for ML training
            println!("Extracting features");
            let features = extract_features(&cwasm_file, &wat_file, &payload, &task.model_folder_name, peak_memory_monitored_kb);
            let csv_path = RESULTS_FOLDER.to_string() + "memory_data.csv";
            if let Err(e) = append_features_to_csv(&csv_path, &features) {
                eprintln!("Failed to append features to CSV: {}", e);
            }

            let peak_memory_estimated = memory_info.estimated_peak_memory_bytes as f64 / (1024.0 * 1024.0);
            match peak_memory_monitored_kb {
                Some(peak_memory_monitored_kb) => {
                    let peak_memory_monitored_mb = peak_memory_monitored_kb as f64 / 1024.0;
                    let data = format!("{},{}, {},{}", &task.task_id.to_string(), &wasm_file, &peak_memory_monitored_mb.to_string(), &peak_memory_estimated.to_string());
                    append_data_to_file(&data, &(RESULTS_FOLDER.to_string()+"memory_results.csv")).unwrap();
                }
                None => {
                    println!("Peak memory monitored: Not available");
                }
            }
        } else {
            println!("Error: Not enough arguments for child process");
        }
        return;
    }
    // Only start HTTP server if this is the parent process
    println!("🚀 HTTP Server starting on http://[::]:8082");
    println!("📡 Available endpoints:");
    println!("   POST /submit_task - Submit a WASM task");
    println!("   GET  /plot_memory - Get memory plots");

    let server = HttpServer::new(move || {
        let mut app = App::new() ;
        app = app.route("/submit_task", web::post().to(handle_submit_task));
        app = app.route("/plot_memory", web::get().to(handle_plot_memory));
        app
    })
    .bind("[::]:8082").unwrap()
    .shutdown_timeout(5) // 5 seconds timeout for graceful shutdown
    .run();

    server.await.unwrap();
}