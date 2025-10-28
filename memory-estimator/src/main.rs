
use std::{env, fs};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::collections::VecDeque;
use std::thread;
use std::time::Duration;
use memory_estimator::memory_info_estimator::{convert_wasm_to_wat, detect_ml_task_from_wat};
use memory_estimator::memory_info_estimator_conservative::{build_memory_info_conservative, MemoryInfoEstimatorConservative};
use memory_estimator::features_extractor::{extract_features, append_features_to_csv};
use memory_estimator::various::append_data_to_file;
use memory_estimator::wasm_loader_basic::run_wasm_job_component_basic;
use memory_estimator::wasm_loader_wasi_nn::run_wasm_job_component_with_wasi_nn;
use serde::{Deserialize, Serialize};
use serde_json;
use base64::{Engine as _, engine::general_purpose};
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use tokio::task;
use core_affinity;

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

// Queue system for handling requests sequentially
#[derive(Debug, Clone)]
pub struct QueuedRequest {
    task: WasmJobRequest,
    timestamp: std::time::SystemTime,
}

#[derive(Debug)]
pub struct RequestQueue {
    queue: VecDeque<QueuedRequest>,
    processing: bool,
    total_processed: u64,
    total_failed: u64,
}

impl RequestQueue {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            processing: false,
            total_processed: 0,
            total_failed: 0,
        }
    }

    pub fn add_request(&mut self, task: WasmJobRequest) {
        let queued_request = QueuedRequest {
            task,
            timestamp: std::time::SystemTime::now(),
        };
        self.queue.push_back(queued_request);
        println!("📥 Request added to queue. Queue size: {}", self.queue.len());
    }

    pub fn get_next_request(&mut self) -> Option<QueuedRequest> {
        self.queue.pop_front()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn mark_processing(&mut self, processing: bool) {
        self.processing = processing;
    }

    pub fn increment_processed(&mut self) {
        self.total_processed += 1;
    }

    pub fn increment_failed(&mut self) {
        self.total_failed += 1;
    }

    pub fn get_stats(&self) -> (usize, bool, u64, u64) {
        (self.queue.len(), self.processing, self.total_processed, self.total_failed)
    }
}

type SharedQueue = Arc<Mutex<RequestQueue>>;

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
    let child = Command::new(current_exe)
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
        println!("Child returned successfully");
    } else {
        println!("Child error: {}", String::from_utf8_lossy(&output.stderr));
        println!("Child stdout: {}", String::from_utf8_lossy(&output.stdout));
    }
}

async fn run_child(task: WasmJobRequest)->(Option<u64>, f64) {
    println!("Child: running wasm job component...");
    
    // Pin the current thread to core 0
    if let Some(cores) = core_affinity::get_core_ids() {
        if let Some(&core_id) = cores.first() {
            if core_affinity::set_for_current(core_id) {
                println!("Pinned thread to core {}", core_id.id);
            } else {
                println!("Failed to pin thread to core {}", core_id.id);
            }
        }
    }

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
    // println!("Payload: {}", payload);
    let component_path = WASM_MODULES_FOLDER.to_string() + &task.binary_name;
    let wat_file_path = WASM_MODULES_FOLDER.to_string() + &task.wat_file;
    println!("Component path: {}", component_path);
    
    // Get memory before WASM execution
    let memory_before_wasm = get_peak_memory_usage();
    if let Some(m) = memory_before_wasm {
        println!("Child: Memory before WASM execution: {} mb", m / 1024);
    }
    let is_ml = detect_ml_task_from_wat(&wat_file_path);
    let start_time = std::time::Instant::now();
    match is_ml {
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
    let duration: f64 = start_time.elapsed().as_secs_f64();
    println!("Child: WASM execution time: {:?}", duration);
    let peak_memory_monitored_kb: Option<u64>= get_peak_memory_usage();

    match (peak_memory_monitored_kb, memory_before_wasm){
        (Some(memory_after), Some(_memory_before)) => {
            println!("Child: Memory after WASM execution: {} MB", memory_after / 1024);        }
        (_, _) => {
            println!("Child: Memory after WASM execution: Not available");
        }
    }

    return (peak_memory_monitored_kb, duration);
}

async fn run_task(task: WasmJobRequest){
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 && args[1] == "child" {
        run_child(task).await;
    } else {
        spawn_child_process(task);
    }

}
async fn handle_submit_task(task: web::Json<WasmJobRequest>, queue: web::Data<SharedQueue>) -> impl Responder {
    let mut queue_guard = queue.lock().unwrap();
    queue_guard.add_request(task.into_inner());
    let queue_size = queue_guard.len();
    drop(queue_guard); // Release the lock
    
    HttpResponse::Ok().json(serde_json::json!({
        "status": "queued",
        "message": "Task added to queue",
        "queue_size": queue_size
    }))
}

async fn handle_plot_memory()->impl Responder{
    HttpResponse::Ok().body("Plots ok")
}

async fn handle_queue_status(queue: web::Data<SharedQueue>) -> impl Responder {
    let queue_guard = queue.lock().unwrap();
    let (queue_size, processing, total_processed, total_failed) = queue_guard.get_stats();
    
    HttpResponse::Ok().json(serde_json::json!({
        "queue_size": queue_size,
        "processing": processing,
        "total_processed": total_processed,
        "total_failed": total_failed,
        "status": if processing { "processing" } else if queue_size > 0 { "waiting" } else { "idle" }
    }))
}

// Background worker function that processes the queue sequentially
fn process_queue_worker(queue: SharedQueue) {
    println!("🔄 Queue worker started");
    
    loop {
        // Check for next request
        let next_request = {
            let mut queue_guard = queue.lock().unwrap();
            if !queue_guard.is_empty() {
                queue_guard.mark_processing(true);
                let request = queue_guard.get_next_request();
                if let Some(ref req) = request {
                    println!("🔄 Processing task: {}", req.task.task_id);
                }
                request
            } else {
                queue_guard.mark_processing(false);
                None
            }
        };

        if let Some(queued_request) = next_request {
            // Process the task
            let task = queued_request.task;
            println!("🚀 Starting task: {}", task.task_id);
            
            // Run the task in a blocking way since we're in a separate thread
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                run_task(task).await;
            });
            
            // Update processed count
            {
                let mut queue_guard = queue.lock().unwrap();
                queue_guard.increment_processed();
                queue_guard.mark_processing(false);
                println!("✅ Task completed. Total processed: {}", queue_guard.total_processed);
            }
        } else {
            // No tasks in queue, wait a bit
            thread::sleep(Duration::from_millis(100));
        }
    }
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

            let (peak_memory_monitored_kb, task_duration) = run_child(task.clone()).await;
            
            if let Some(peak_memory_monitored) = peak_memory_monitored_kb {
                let peak_memory_monitored = peak_memory_monitored as f64 /  1024.0;
                println!("Peak memory monitored: {} MB", peak_memory_monitored);
            } else {
                println!("Peak memory monitored: Not available");
            }

            // Extract features and store row for ML training
            println!("Extracting features");
            let features = extract_features(&cwasm_file, &wat_file, &payload, &task.model_folder_name, peak_memory_monitored_kb, task_duration);
            let csv_path = RESULTS_FOLDER.to_string() + "memory_data.csv";
            if let Err(e) = append_features_to_csv(&csv_path, &features) {
                eprintln!("Failed to append features to CSV: {}", e);
            }

            let peak_memory_estimated = memory_info.estimated_peak_memory_bytes as f64 / (1024.0 * 1024.0);
            match peak_memory_monitored_kb {
                Some(peak_memory_monitored_kb) => {
                    let peak_memory_monitored_mb = peak_memory_monitored_kb as f64 / 1024.0;
                    // let data = format!("{}, {}, {}, {}, {}", &task.task_id.to_string(), &wasm_file, &peak_memory_monitored_mb.to_string(), &peak_memory_estimated.to_string(), task_duration.to_string());
                    // append_data_to_file(&data, &(RESULTS_FOLDER.to_string()+"memory_results.csv")).unwrap();
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
    println!("   GET  /queue_status - Get queue status");

    // Create shared queue
    let queue: SharedQueue = Arc::new(Mutex::new(RequestQueue::new()));
    
    // Clone for background worker
    let worker_queue = queue.clone();
    
    // Start background worker in a separate thread
    let _worker_handle = thread::spawn(move || {
        process_queue_worker(worker_queue);
    });
    
    // Create HTTP server with queue data
    let server = HttpServer::new(move || {
        let mut app = App::new()
            .app_data(web::Data::new(queue.clone()));
        app = app.route("/submit_task", web::post().to(handle_submit_task));
        app = app.route("/plot_memory", web::get().to(handle_plot_memory));
        app = app.route("/queue_status", web::get().to(handle_queue_status));
        app
    })
    .bind("[::]:8082").unwrap()
    .shutdown_timeout(5) // 5 seconds timeout for graceful shutdown
    .run();

    // Start server
    let server_handle = tokio::spawn(async move {
        server.await.unwrap();
    });

    // Wait for server to finish
    server_handle.await.unwrap();
    
    // Note: The worker thread will continue running until the process exits
    // In a production system, you might want to add graceful shutdown handling
}