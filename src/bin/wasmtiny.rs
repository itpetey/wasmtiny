use anyhow::Result;
use clap::Parser;
use wasmtiny::{WasmApplication, WasmValue};

#[derive(Parser, Debug)]
struct Args {
    #[arg(help = "Path to WASM module")]
    module: String,

    #[arg(short, long, help = "Function to call")]
    function: Option<String>,

    #[arg(short, long, help = "Arguments to pass to the function")]
    args: Vec<i32>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let mut app = WasmApplication::new();

    let module_idx = app.load_module_from_file(&args.module)?;

    println!("Loaded WASM module from {}", &args.module);

    match args.function {
        Some(func) => {
            let wasm_args: Vec<WasmValue> = args.args.iter().map(|&i| WasmValue::I32(i)).collect();

            match app.call_function(module_idx, &func, &wasm_args) {
                Ok(results) => {
                    println!("Function '{}' returned: {:?}", &func, results);
                }
                Err(e) => {
                    eprintln!("Error calling function '{}': {}", &func, e);
                    std::process::exit(1);
                }
            }
        }
        None => match app.execute_main(module_idx, &[]) {
            Ok(_) => {
                println!("Module executed successfully");
            }
            Err(e) => {
                eprintln!("Error executing module: {}", e);
                std::process::exit(1);
            }
        },
    }

    Ok(())
}
