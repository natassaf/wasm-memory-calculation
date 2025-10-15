
use std::env;
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use memory_estimator::memory_info_estimator::{build_memory_info, convert_wasm_to_wat, print_memory_analysis_simple, MemoryInfoEstimator};
use memory_estimator::wasm_loaders::run_wasm_job_component;
use serde::{Deserialize, Serialize};
use serde_json;
use base64::{Engine as _, engine::general_purpose};
use actix_web::{web, App, HttpResponse, HttpServer, Responder};



#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WasmJobRequest{
    binary_name: String,
    func_name: String,
    payload: String,
    payload_compressed: bool,
    task_id: usize,
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
    
    let mut child = Command::new(std::env::current_exe().unwrap())
        .arg("child")
        .arg(&task_file) // Only pass the task file path
        .env("OMP_NUM_THREADS", "1")
        .env("MKL_NUM_THREADS", "1")
        .env("NUMEXPR_NUM_THREADS", "1")
        .env("OPENBLAS_NUM_THREADS", "1")
        .env("BLIS_NUM_THREADS", "1")
        .env("VECLIB_MAXIMUM_THREADS", "1")
        .env("NUMBA_NUM_THREADS", "1")
        .env("ORT_DISABLE_PARALLELISM", "1")
        .env("ORT_NUM_THREADS", "1")
        .env("ORT_EXECUTION_PROVIDER", "CPUExecutionProvider")
        .env("MKL_DYNAMIC", "FALSE")
        .env("OMP_DYNAMIC", "FALSE")
        .env("OPENBLAS_DYNAMIC", "FALSE")
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

async fn run_child(task: WasmJobRequest) {
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

    // Run WASM component with error handling
    match run_wasm_job_component(
        task.task_id,
        "wasm-modules/".to_string() + &task.binary_name,
        task.func_name,
        payload,
        task.model_folder_name,
    ).await {
        Ok(result) => println!("Child result: {:?}", result),
        Err(e) => println!("Child error: {:?}", e),
    }

    #[cfg(target_os = "linux")]
    {
        if let Ok(statm_content) = std::fs::read_to_string("/proc/self/statm") {
            let parts: Vec<&str> = statm_content.trim().split_whitespace().collect();
            if parts.len() >= 2 {
                let rss_pages: u64 = parts[1].parse().unwrap_or(0);
                let rss_kb = rss_pages * 4; // Each page is 4KB on Linux
                println!("Child: /proc/self/statm RSS = {} KB ({} pages)", rss_kb, rss_pages);
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(status_content) = std::fs::read_to_string("/proc/self/status") {
            for line in status_content.lines() {
                if line.starts_with("VmRSS:") || line.starts_with("VmSize:") || line.starts_with("VmPeak:") {
                    println!("Child: {}", line.trim());
                }
            }
        }
    }

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
    // Set environment variables BEFORE any WASM execution to force single-threaded behavior
    std::env::set_var("OMP_NUM_THREADS", "1");
    std::env::set_var("MKL_NUM_THREADS", "1");
    std::env::set_var("NUMEXPR_NUM_THREADS", "1");
    std::env::set_var("OPENBLAS_NUM_THREADS", "1");
    std::env::set_var("BLIS_NUM_THREADS", "1");
    std::env::set_var("VECLIB_MAXIMUM_THREADS", "1");
    std::env::set_var("NUMBA_NUM_THREADS", "1");
    std::env::set_var("ORT_DISABLE_PARALLELISM", "1");
    std::env::set_var("ORT_NUM_THREADS", "1");
    std::env::set_var("ORT_EXECUTION_PROVIDER", "CPUExecutionProvider");
    std::env::set_var("MKL_DYNAMIC", "FALSE");
    std::env::set_var("OMP_DYNAMIC", "FALSE");
    std::env::set_var("OPENBLAS_DYNAMIC", "FALSE");
    
    let args: Vec<String> = env::args().collect();
    // If this is a child process, run the WASM task and exit
    if args.len() > 1 && args[1] == "child" {
        // Parse command line arguments for child process
        if args.len() >= 3 {
            let task_file = &args[2];
            let task_json = std::fs::read_to_string(task_file).expect("Failed to read task file");
            let task: WasmJobRequest = serde_json::from_str(&task_json).expect("Failed to parse task JSON");
            
            // Construct full file paths
            let cwasm_file: String = "wasm-modules/".to_string() + &task.cwasm_file;
            let wat_file: String = "wasm-modules/".to_string() + &task.wat_file;
            let wasm_file: String = "wasm-modules/".to_string() + &task.binary_name;
            // Convert WASM to WAT only if .wat file doesn't exist
            if !std::path::Path::new(&wat_file).exists() {
                match convert_wasm_to_wat(&wasm_file, &wat_file) {
                    Ok(_) => println!("Successfully converted {} to {}", cwasm_file, wat_file),
                    Err(e) => println!("Error converting file: {}", e),
                }
            }

            let memory_info: MemoryInfoEstimator = build_memory_info(&cwasm_file, &wat_file);
            println!("Estimated memory info: {}", memory_info);
            print_memory_analysis_simple(&memory_info);
            
            run_child(task).await;
        } else {
            println!("Error: Not enough arguments for child process");
        }
        return; // Exit child process - don't start HTTP server
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