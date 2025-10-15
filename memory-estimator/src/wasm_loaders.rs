
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


impl WasmComponentLoader{
    pub fn new(folder_to_mount:String)->Self{
        println!("Loading wasm component");

        // initialize engine
        let mut config = Config::new();
        config.async_support(true).wasm_component_model(true);
        
        // Disable ALL threading-related features to prevent mutex issues
        config.wasm_threads(false);
        config.wasm_multi_memory(false);
        
        // Disable additional features that might cause threading issues
        config.wasm_reference_types(true);
        config.wasm_bulk_memory(true);
        
        // Disable parallel compilation to avoid threading issues
        config.parallel_compilation(false);
        
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



        // Initialize ONNX backend with single-threaded configuration
        let mut onnx_backend = OnnxBackend::default();
        
        // Set ALL possible environment variables to force single-threaded execution
        std::env::set_var("OMP_NUM_THREADS", "1");
        std::env::set_var("MKL_NUM_THREADS", "1");
        std::env::set_var("NUMEXPR_NUM_THREADS", "1");
        std::env::set_var("OPENBLAS_NUM_THREADS", "1");
        std::env::set_var("BLIS_NUM_THREADS", "1");
        std::env::set_var("VECLIB_MAXIMUM_THREADS", "1");
        std::env::set_var("NUMBA_NUM_THREADS", "1");
        std::env::set_var("NUMBA_NUM_NUM_THREADS", "1");
        std::env::set_var("NUMBA_NUM_NUM_NUM_THREADS", "1");
        
        // ONNX Runtime specific settings
        std::env::set_var("ORT_DISABLE_PARALLELISM", "1");
        std::env::set_var("ORT_NUM_THREADS", "1");
        std::env::set_var("ORT_EXECUTION_PROVIDER", "CPUExecutionProvider");
        
        // Additional threading control
        std::env::set_var("MKL_DYNAMIC", "FALSE");
        std::env::set_var("OMP_DYNAMIC", "FALSE");
        std::env::set_var("OPENBLAS_DYNAMIC", "FALSE");
        
        let onnx_backend = Backend::from(onnx_backend);
        
        
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
}

pub async fn run_wasm_job_component(task_id: usize, component_name:String, func_name:String, payload:String, folder_to_mount:String)->Result<Vec<Val>, Error>{
    // Set up Wasmtime engine and module outside blocking
    // let component_name ="math_tasks".to_string();
    let folder_to_mount = "models".to_string();
    let mut shared_wasm_loader = WasmComponentLoader::new(folder_to_mount);
    // Use the shared wasm_loader instead of creating a new one
    let func_to_run: wasmtime::component::Func = shared_wasm_loader.load_func(component_name, func_name).await;
    let input = vec![input_to_wasm_event_val(payload)];

    let result: Result<Vec<Val>, anyhow::Error> = shared_wasm_loader.run_func(input, func_to_run).await;
    match &result {
        Ok(val) => {},
        Err(e) => println!("error: {:?}", e),
    }
    println!("Finished wasm task {}", task_id);
    
    // Return the result instead of unwrapping
    match result {
        Ok(val) => Ok(val),
        Err(e) => Err(e)
    }
    }

fn input_to_wasm_event_val(input:String) -> wasmtime::component::Val {
    let event_val = wasmtime::component::Val::String(input.into());
    let record_fields = vec![
        ("event".to_string(), event_val)
    ];
    wasmtime::component::Val::Record(record_fields.into())
}
