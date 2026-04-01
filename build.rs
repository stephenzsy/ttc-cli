use std::env;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if env::var("PROTOC").is_ok() && env::var("PROTOC").unwrap() != "" {
        println!("PROTOC environment variable is set, continue with code generation.");
        prost_build::compile_protos(&["./src/protos/gtfs-realtime.proto"], &["./src"])?;
        let output_path = Path::new(&env::var("OUT_DIR")?).join("transit_realtime.rs");
        let target_spec_path = Path::new("./src/protos/transit_realtime.rs");
        std::fs::copy(output_path, target_spec_path)?;
    } else {
        println!("PROTOC environment variable is not set, skipping code generation.");
    }

    Ok(())
}
