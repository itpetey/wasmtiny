use clap::Parser;
use wasmtiny::{WasmApplication, WasmValue};

#[derive(Parser, Debug)]
#[command(name = "wasmtiny")]
#[command(about = "A tiny WebAssembly runtime")]
struct Args {
    #[arg(help = "Path to WASM file")]
    wasm_file: Option<String>,

    #[arg(short, long, help = "Function to call")]
    function: Option<String>,

    #[arg(short, long, help = "Arguments to pass to the function")]
    args: Vec<i32>,
}

fn main() {
    let args = Args::parse();

    match &args.wasm_file {
        Some(file) => {
            let mut app = WasmApplication::new();

            match app.load_module_from_file(file) {
                Ok(module_idx) => {
                    println!("Loaded WASM module from {}", file);

                    if let Some(func_name) = &args.function {
                        let wasm_args: Vec<WasmValue> =
                            args.args.iter().map(|&i| WasmValue::I32(i)).collect();

                        match app.call_function(module_idx, func_name, &wasm_args) {
                            Ok(results) => {
                                println!("Function '{}' returned: {:?}", func_name, results);
                            }
                            Err(e) => {
                                eprintln!("Error calling function '{}': {}", func_name, e);
                                std::process::exit(1);
                            }
                        }
                    } else {
                        match app.execute_main(module_idx, &[]) {
                            Ok(_) => {
                                println!("Module executed successfully");
                            }
                            Err(e) => {
                                eprintln!("Error executing module: {}", e);
                                std::process::exit(1);
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error loading WASM module: {}", e);
                    std::process::exit(1);
                }
            }
        }
        None => {
            eprintln!("Error: No WASM file specified");
            eprintln!("Usage: wasmtiny <wasm_file> [--function <name>] [--args <args>...]");
            std::process::exit(1);
        }
    }
}
