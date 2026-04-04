use clap::Parser;
use reqwest::Url;
use std::{error::Error, fs, io::Cursor};

pub mod db;

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

const GTFS_DATA_BASE_URL: &str = "https://ckan0.cf.opendata.inter.prod-toronto.ca";

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
    let archive_file_name = data_dir_path.join("gfts.zip");
    if !args.skip_download {
        // download metadata
        let mut metadata_url =
            Url::parse(&format!("{}/api/3/action/package_show", GTFS_DATA_BASE_URL))?;
        metadata_url
            .query_pairs_mut()
            .append_pair("id", "merged-gtfs-ttc-routes-and-schedules");
        let metadata = reqwest::get(metadata_url)
            .await?
            .json::<serde_json::Value>()
            .await?;
        let metadata_file = fs::File::create(data_dir_path.join("metadata.json"))?;
        serde_json::to_writer_pretty(metadata_file, &metadata)?;

        let resource_url = metadata["result"]["resources"][0]["url"].as_str();
        if let Some(url) = resource_url {
            let response = reqwest::get(url).await?;
            let mut file = std::fs::File::create(&archive_file_name)?;
            let mut content = Cursor::new(response.bytes().await?);
            std::io::copy(&mut content, &mut file)?;
        } else {
            return Err(String::from("Resource URL not found in metadata").into());
        }
    }

    // prepare sqlite connection
    let gtfs_db = db::GtfsDb::new(&data_dir_path.as_path(), true)?;
    gtfs_db.initialize()?;

    let mut archive_file: zip::ZipArchive<fs::File> =
        zip::ZipArchive::new(fs::File::open(&archive_file_name)?)?;
    let stops_csv = archive_file.by_name("stops.txt")?;
    let mut rdr = csv::Reader::from_reader(stops_csv);
    for result in rdr.deserialize() {
        // The iterator yields Result<StringRecord, Error>, so we check the
        // error here.
        let record: db::Stop = result?;
        gtfs_db.insert_stop(record)?;
    }

    Ok(())
}
