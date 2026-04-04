use clap::Parser;
use std::path::Path;
use std::{env, error::Error, fs};

#[derive(clap::Parser, Debug)]
#[command(version)]
struct Args {
    #[arg(short, long, default_value_t = false)]
    update: bool,

    #[arg(short, long)]
    data_dir: Option<String>,

    #[arg(long, default_value_t = false, requires = "update")]
    download: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let data_dir = args.data_dir.unwrap_or_else(|| {
        let parent = match env::var("HOME") {
            Ok(v) => v,
            Err(_) => ".".to_string(),
        };
        parent + "/" + ".ttc"
    });
    let data_dir_path = Path::new(&data_dir);

    if !args.update {
        if !data_dir_path.is_dir() {
            println!("Data directory has not been initialized")
        }
    } else {
        if !data_dir_path.exists() {
            fs::create_dir_all(data_dir_path)?;
            println!("Created data directory: {:?}", data_dir_path.to_str())
        }
    }

    Ok(())
}
