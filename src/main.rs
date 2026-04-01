use chrono::{DateTime, Local};
use clap::Parser;
use csv::StringRecord;
use prost::Message;
use reqwest::{self, Url};
use std::{error::Error, io::Read};

pub mod transit_realtime {
    include!("protos/transit_realtime.rs");
}

const URL: &str = "https://gtfsrt.ttc.ca/trips/update?format=binary";
const GTFS_DATA_BASE_URL: &str = "https://ckan0.cf.opendata.inter.prod-toronto.ca";

#[derive(Parser, Debug)]
struct Filter {
    #[arg(short, long)]
    route: String,

    #[arg(short, long)]
    stop: String,
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
    zip::ZipArchive::new(std::io::Cursor::new(archive_bytes))?
        .by_name("stops.txt")?
        .read_to_end(&mut buf)?;
    std::fs::write("./stops.txt", buf)?;
    Ok(())
}

fn lookup_stop_code(stop_code: &String) -> Result<Option<StringRecord>, Box<dyn Error>> {
    let mut rdr = csv::Reader::from_path("./stops.txt")?;
    for result in rdr.records() {
        // The iterator yields Result<StringRecord, Error>, so we check the
        // error here.
        let record = result?;

        if record.get(1).unwrap() == stop_code {
            return Ok(Some(record));
        }
    }
    Ok(None)
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

    let stop_id_result = lookup_stop_code(&filter.stop)?;
    if stop_id_result.is_none() {
        println!("Stop code {} not found", &filter.stop);
        return Ok(());
    }
    let stop_record = stop_id_result.unwrap();
    let arg_stop_id = stop_record.get(0).unwrap();
    let arg_stop_name = stop_record.get(2).unwrap();
    println!("Stop Name: {}", arg_stop_name);

    let feed_raw = reqwest::get(URL).await?.bytes().await?;
    // pull feed
    let message = transit_realtime::FeedMessage::decode(feed_raw)?;

    println!(
        "Feed timestamp: {}",
        timestamp_to_local(message.header.timestamp.unwrap() as i64).unwrap()
    );

    // filter route
    let filtered = message
        .entity
        .iter()
        .filter_map(|entity| {
            if !entity.is_deleted()
                && let Some(trip_update) = &entity.trip_update
                && let Some(route_id) = &trip_update.trip.route_id
                && route_id == &filter.route
            {
                let filtered_stop_time_updates = trip_update
                    .stop_time_update
                    .iter()
                    .filter_map(|stop_time_update| {
                        if let Some(stop_id) = &stop_time_update.stop_id {
                            if *stop_id == arg_stop_id {
                                return Some(stop_time_update.clone());
                            }
                        }
                        None
                    })
                    .collect::<Vec<_>>();
                if filtered_stop_time_updates.len() == 0 {
                    return None;
                }
                let mut trip_update = trip_update.clone();
                trip_update.stop_time_update = filtered_stop_time_updates;
                return Some(trip_update);
            }
            None
        })
        .collect::<Vec<_>>();
    if args.debug {
        println!("Filtered Trip Updates: {:#?}", filtered);
    }
    for trip_update in filtered {
        let vehicle_id = trip_update.vehicle.unwrap().id.unwrap();
        println!("------------------------------");
        println!("Vehicle: {}", vehicle_id);
        for stop_time_update in trip_update.stop_time_update {
            if let Some(arrival) = stop_time_update.arrival
                && let Some(time) = arrival.time
            {
                println!("  Time Arrival: {}", timestamp_to_local(time).unwrap());
            } else {
                println!("  Time Arrival: N/A");
            }
        }
    }

    Ok(())
}
