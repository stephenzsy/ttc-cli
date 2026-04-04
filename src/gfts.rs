use clap::Parser;
use std::{error::Error, fs};

mod db;

#[derive(clap::Parser, Debug)]
#[command(version)]
struct Args {
    #[arg(short, long, default_value_t = false)]
    update: bool,

    #[arg(short, long)]
    data_dir: Option<String>,

    #[arg(long, default_value_t = false, requires = "update")]
    skip_download: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let data_dir_path = db::resolve_data_dir(&args.data_dir);

    if !args.update {
        if !&data_dir_path.is_dir() {
            return Err(String::from("Data directory has not been initialized").into());
        }
        return Ok(());
    }

    if !&data_dir_path.exists() {
        fs::create_dir_all(&data_dir_path)?;
        println!("Created data directory: {:?}", &data_dir_path)
    }
    if !args.skip_download {
        
    }

    Ok(())
}
