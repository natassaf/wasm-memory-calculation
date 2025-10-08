
use anyhow::{anyhow, Context};
use serde::{Deserialize, Serialize};
use wasmtime::*;
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::p2::{self, IoView, WasiCtx, WasiCtxBuilder, WasiView};
use wasmtime_wasi_nn::wit::{add_to_linker as add_wasi_nn};
use wasmtime_wasi_nn::wit::{ WasiNnCtx, WasiNnView};
use wasmtime::component::{Component, Func, Linker, Val};
use wasmtime_wasi_nn::{InMemoryRegistry, Registry};
use wasmtime_wasi_nn::Backend;
use wasmtime_wasi::{DirPerms, FilePerms};
use wasmtime_wasi_nn::backend::onnx::OnnxBackend;

// pub struct ModuleWasmLoader{
//     engine:Engine,
//     //  pub store: Store<WasiP1Ctx>,
//     pub store: Store<()>,
//     pub linker: Option<Linker<WasiP1Ctx>>,
// }

// impl ModuleWasmLoader{

//     // pub fn new_async(data:())->Self{
//     //     println!("Loading wasm module");
//     //     let args = std::env::args().skip(1).collect::<Vec<_>>();
//     //     let mut config = Config::new();
//     //     config.async_support(true);
//     //     let engine = Engine::new(&config).unwrap();
//     //     let mut linker: Linker<WasiP1Ctx> = Linker::new(&engine);
//     //     preview1::add_to_linker_async(&mut linker, |t| t);

//     //     let wasi_ctx = WasiCtxBuilder::new()
//     //         .inherit_stdio()
//     //         .inherit_env()
//     //         .args(&args)
//     //         .build_p1();

//     //     let  store = Store::new(&engine, wasi_ctx);
//     //     Self {engine, store, linker}
//     // }

//     pub fn new(data:())->Self{
//         println!("Loading wasm module");
//         let engine = Engine::default();
//         let mut store = Store::new(&engine, ());
//         Self {engine, store, linker:None}
//     }

    
//     pub fn load<I:WasmParams, O:WasmResults>(&mut self, path_to_module:String, func_name:String)->(TypedFunc<I, O>, Memory ){
//         let module = Module::from_file(&self.engine, path_to_module).unwrap();
//         let instance = Instance::new(&mut self.store, &module, &[]).unwrap();
//         // let module = Module::from_file(&self.engine, path_to_module).unwrap();
//         // let pre = self.linker.instantiate_pre(&module).unwrap();
//         // let instance = pre.instantiate(&mut self.store).unwrap();
//         // let instance = self.linker.instantiate_async(&mut self.store, &module).await.unwrap();

//         // memory stuff
//         let memory = instance.get_memory(&mut self.store, "memory").expect("No `memory` export found in Wasm module");
//         let loaded_func = instance.get_typed_func::<I, O>(&mut self.store, &func_name).unwrap();
//         (loaded_func, memory)
//     }
// }

struct HostState {
    wasi: WasiCtx,
    table: wasmtime::component::ResourceTable,
    wasi_nn: WasiNnCtx,
}

impl HostState {
    fn wasi_nn_view(&mut self) -> WasiNnView<'_> {
        WasiNnView::new(&mut self.table, &mut self.wasi_nn)
    }
}

impl WasiView for HostState {
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.wasi
    }
}
impl IoView for HostState {
    fn table(&mut self) -> &mut wasmtime::component::ResourceTable {
        &mut self.table
    }
}


pub struct WasmComponentLoader{
    engine:Engine,
    //  pub store: Store<WasiP1Ctx>,
    store: Store<HostState>,
    linker: Linker<HostState>,
}

#[derive(Serialize, Deserialize, Debug)]
struct WasmResult {
    output: String,
}

#[derive(Debug, Clone)]
pub struct MemoryMonitor {
    pub peak_memory_bytes: u64,
    pub initial_memory_bytes: u64,
    pub final_memory_bytes: u64,
    pub execution_time_ms: u64,
}

impl MemoryMonitor {
    pub fn new() -> Self {
        Self {
            peak_memory_bytes: 0,
            initial_memory_bytes: 0,
            final_memory_bytes: 0,
            execution_time_ms: 0,
        }
    }

    pub fn get_current_memory_usage() -> u64 {
        // Try Linux /proc/self/statm first (most accurate for Linux)
        if let Ok(statm) = std::fs::read_to_string("/proc/self/statm") {
            if let Some(second_field) = statm.split_whitespace().nth(1) {
                if let Ok(pages) = second_field.parse::<u64>() {
                    return pages * 4096; // Assuming 4KB pages
                }
            }
        }
        
        // For macOS and other Unix-like systems, use ps command
        // This works on macOS, FreeBSD, and other Unix systems
        let process = std::process::Command::new("ps")
            .args(&["-o", "rss=", "-p", &std::process::id().to_string()])
            .output();
            
        if let Ok(output) = process {
            if let Ok(rss_str) = String::from_utf8(output.stdout) {
                if let Ok(rss_kb) = rss_str.trim().parse::<u64>() {
                    return rss_kb * 1024; // Convert KB to bytes
                }
            }
        }
        
        // Fallback: return a reasonable estimate
        50 * 1024 * 1024 // 50MB fallback
    }
}


impl WasmComponentLoader{
    pub fn new(folder_to_mount:String)->Self{
        println!("Loading wasm component");

        // initialize engine
        let mut config = Config::new();
        config.async_support(true).wasm_component_model(true);
        let engine = Engine::new(&config).unwrap();

        // initialize linker
        let mut linker: Linker<HostState> = Linker::new(&engine);
        p2::add_to_linker_async(&mut linker).context("add_to_linker_async failed").unwrap();
        // Add wasm-nn support
        add_wasi_nn(&mut linker, |host: &mut HostState| {
            HostState::wasi_nn_view(host)
        }).context("failed to add wasi-nn to linker").unwrap();
       let wasi = match folder_to_mount.as_str() {
            "" => {WasiCtxBuilder::new()
            .inherit_stdio()
            .build()}
            _ => {WasiCtxBuilder::new()
            .inherit_stdio()
            .preopened_dir(
                folder_to_mount.clone(),   // host path
                folder_to_mount.clone(),  // guest path
                DirPerms::READ,
                FilePerms::READ,
            )
            .unwrap()
            .build()
            }};



        // Initialize ONNX backend
        let onnx_backend = Backend::from(OnnxBackend::default());
        
        
        let my_registry = InMemoryRegistry::new();
        let registry = Registry::from(my_registry);
        
        // Create the WasiNnCtx with the ONNX backend
        let wasi_nn = WasiNnCtx::new(vec![onnx_backend], registry);

        let store: Store<HostState> = Store::new(
            &engine,
            HostState {
                wasi,
                table: wasmtime::component::ResourceTable::new(),
                wasi_nn,
            },
        );

        Self {engine, store, linker}
    }

    pub async fn load_func(&mut self, wasm_component_path:String, func_name:String)->Func{
        let component = Component::from_file(&self.engine, wasm_component_path.clone())
        .with_context(|| format!("failed to compile component at {:?}", wasm_component_path)).unwrap();

        // 4) Instantiate
        let instance = self.linker.instantiate_async(&mut self.store, &component)
        .await
        .context("instantiate_async failed").unwrap();

        // 5) Lookup exported function by its world export name (usually the same as in the WIT).
        let func: Func = instance
            .get_func(&mut self.store, &func_name)
            .ok_or_else(|| anyhow!("exported function `{func_name}` not found")).unwrap();

        return func;
    }

    pub async fn run_func(&mut self, input:Vec<Val>, func:Func)->Result<Vec<Val>, anyhow::Error>{
        let results_len = func.results(&self.store).len();
        
        // Initialize with empty string for WasmResult output
        let mut results = vec![Val::String("".into()); results_len];

        let input_args = input;
        func.call_async(&mut self.store, &input_args, &mut results).await?;
        // println!("load result {:?}", results);
        return Ok(results)
    }

    pub async fn run_func_with_memory_monitoring(&mut self, input:Vec<Val>, func:Func)->Result<(Vec<Val>, MemoryMonitor), anyhow::Error>{
        let results_len = func.results(&self.store).len();
        let mut monitor = MemoryMonitor::new();
        
        // Record initial memory usage
        monitor.initial_memory_bytes = MemoryMonitor::get_current_memory_usage();
        println!("🔍 Initial memory usage: {} bytes ({:.2} MB)", 
                monitor.initial_memory_bytes, 
                monitor.initial_memory_bytes as f64 / (1024.0 * 1024.0));
        
        let start_time = std::time::Instant::now();
        println!("🚀 Starting WASM execution...");
        
        // Initialize with empty string for WasmResult output
        let mut results = vec![Val::String("".into()); results_len];
        let input_args = input;
        
        // Execute the function
        func.call_async(&mut self.store, &input_args, &mut results).await?;
        
        // Record final memory usage and execution time
        monitor.final_memory_bytes = MemoryMonitor::get_current_memory_usage();
        monitor.execution_time_ms = start_time.elapsed().as_millis() as u64;
        monitor.peak_memory_bytes = monitor.final_memory_bytes.max(monitor.initial_memory_bytes);
        
        println!("⏱️  Execution time: {} ms", monitor.execution_time_ms);
        println!("🔍 Final memory usage: {} bytes ({:.2} MB)", 
                monitor.final_memory_bytes, 
                monitor.final_memory_bytes as f64 / (1024.0 * 1024.0));
        println!("📊 Peak memory usage: {} bytes ({:.2} MB)", 
                monitor.peak_memory_bytes, 
                monitor.peak_memory_bytes as f64 / (1024.0 * 1024.0));
        
        return Ok((results, monitor))
    }
}

fn input_to_wasm_event_val(input:String) -> wasmtime::component::Val {
    let event_val = wasmtime::component::Val::String(input.into());
    let record_fields = vec![
        ("event".to_string(), event_val)
    ];
    wasmtime::component::Val::Record(record_fields.into())
}

pub async fn run_wasm_job_component(task_id: usize, component_name:String, func_name:String, payload:String, folder_to_mount:String)->Result<Vec<Val>, Error>{
    // Set up Wasmtime engine and module outside blocking
    // let component_name ="math_tasks".to_string();
    let folder_to_mount = "models".to_string();
    println!("Running component {:?}, func: {:?}", component_name, func_name);
    let mut shared_wasm_loader = WasmComponentLoader::new(folder_to_mount);
    // Use the shared wasm_loader instead of creating a new one
    let func_to_run: wasmtime::component::Func = shared_wasm_loader.load_func(component_name, func_name).await;
    println!("payload: {:?}", payload);
    let input = vec![input_to_wasm_event_val(payload)];

    let result: Result<Vec<Val>, anyhow::Error> = shared_wasm_loader.run_func(input, func_to_run).await;
    match &result {
        Ok(val) => println!("result: {:?}", val),
        Err(e) => println!("error: {:?}", e),
    }
    println!("Finished wasm task {}", task_id);
    
    // Return the result instead of unwrapping
    match result {
        Ok(val) => Ok(val),
        Err(e) => Err(e)
    }
    
}

pub async fn run_wasm_job_component_with_memory_monitoring(task_id: usize, component_name:String, func_name:String, payload:String, folder_to_mount:String)->Result<(Vec<Val>, MemoryMonitor), Error>{
    let folder_to_mount = "models".to_string();
    println!("Running component {:?}, func: {:?}", component_name, func_name);
    let mut shared_wasm_loader = WasmComponentLoader::new(folder_to_mount);
    let func_to_run: wasmtime::component::Func = shared_wasm_loader.load_func(component_name, func_name).await;
    println!("payload: {:?}", payload);
    let input = vec![input_to_wasm_event_val(payload)];

    let result: Result<(Vec<Val>, MemoryMonitor), anyhow::Error> = shared_wasm_loader.run_func_with_memory_monitoring(input, func_to_run).await;
    match &result {
        Ok((val, monitor)) => {
            println!("result: {:?}", val);
            println!("Finished wasm task {} with memory monitoring", task_id);
        },
        Err(e) => println!("error: {:?}", e),
    }
    
    match result {
        Ok((val, monitor)) => Ok((val, monitor)),
        Err(e) => Err(e)
    }
}
