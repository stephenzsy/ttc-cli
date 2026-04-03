use chrono::{DateTime, Local};
use clap::Parser;
use reqwest::{self, Url};
use std::{error::Error, io::Read};

pub mod transit_realtime {
    include!("protos/transit_realtime.rs");
}

const GTFS_DATA_BASE_URL: &str = "https://ckan0.cf.opendata.inter.prod-toronto.ca";

#[derive(Parser, Debug)]
struct Filter {
    #[arg(short, long)]
    route: String,

    #[arg(short, long)]
    stop: String,

    #[arg(long, default_value_t = false)]
    show_scheduled: bool,
}

#[derive(clap::Parser, Debug)]
#[command(version)]
struct Args {
    #[command(flatten)]
    filter: Option<Filter>,

    #[arg(
        long,
        default_value_t = false,
        conflicts_with = "route",
        conflicts_with = "stop"
    )]
    update_stops: bool,

    #[arg(long, default_value_t = false)]
    debug: bool,
}

async fn update_stops() -> Result<(), Box<dyn Error>> {
    let mut metadata_url =
        Url::parse(&format!("{}/api/3/action/package_show", GTFS_DATA_BASE_URL))?;
    metadata_url
        .query_pairs_mut()
        .append_pair("id", "merged-gtfs-ttc-routes-and-schedules");
    let metadata = reqwest::get(metadata_url)
        .await?
        .json::<serde_json::Value>()
        .await?;
    println!("Downloading GFTS archive");
    let archive_bytes = reqwest::get(metadata["result"]["resources"][0]["url"].as_str().unwrap())
        .await?
        .bytes()
        .await?;
    let mut buf: Vec<u8> = Vec::new();
    zip::ZipArchive::new(std::io::Cursor::new(&archive_bytes.clone()))?
        .by_name("stops.txt")?
        .read_to_end(&mut buf)?;
    std::fs::write("./stops.txt", buf)?;
    let mut buf: Vec<u8> = Vec::new();
    zip::ZipArchive::new(std::io::Cursor::new(&archive_bytes.clone()))?
        .by_name("stop_times.txt")?
        .read_to_end(&mut buf)?;
    std::fs::write("./stop_times.txt", buf)?;
    Ok(())
}

fn timestamp_to_local(timestamp: i64) -> Option<DateTime<Local>> {
    DateTime::from_timestamp(timestamp, 0).map(|dt| dt.with_timezone(&Local))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    if args.update_stops {
        update_stops().await?;
        return Ok(());
    }

    let filter = args
        .filter
        .ok_or("Filter is required when not updating stops")?;

    let session = ttc_cli::TTCRealTime::new(None)?;
    let next_bus = session.next_bus(filter.route, filter.stop).await?;

    if let Some(timestamp) = next_bus.timestamp
        && let Some(local_ts) = timestamp_to_local(timestamp as i64)
    {
        println!("Feed Timestamp: {}", local_ts);
    } else {
        println!("Feed Timestamp: N/A");
    }
    println!("Stop Name: {}", next_bus.stop_name);
    if next_bus.trips.len() == 0 {
        println!("No upcoming trips found for the specified route and stop.");
    } else {
        for trip in next_bus.trips {
            if trip.arrival_times.len() == 0 {
                continue;
            }
            println!("--------------------------------");
            /*
            if let Some(trip_id) = &trip.trip_id {
                println!("  Trip ID: {}", trip_id);
            }
            */
            if let Some(vehicle_id) = &trip.vehicle_id {
                println!("  Vehicle ID: {}", vehicle_id);
            } else {
                println!("  Vehicle ID: N/A");
            }
            for arrival_time in trip.arrival_times {
                if let Some(arrival_time) = arrival_time
                    && let Some(local_arrival) = timestamp_to_local(arrival_time)
                {
                    println!("  Arrival Time: {}", local_arrival);
                } else {
                    println!("  Arrival Time: N/A");
                }
            }
        }
    }
    Ok(())
}
