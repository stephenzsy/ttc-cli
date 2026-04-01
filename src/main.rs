use chrono::{DateTime, Local};
use clap::Parser;
use prost::Message;
use reqwest;
use std::error::Error;

pub mod transit_realtime {
    include!("protos/transit_realtime.rs");
}

const URL: &str = "https://gtfsrt.ttc.ca/trips/update?format=binary";

#[derive(clap::Parser, Debug)]
struct Args {
    #[arg(short, long)]
    route: String,

    #[arg(short, long)]
    stop: String,
}

fn lookup_stop_code(stop_code: &String) -> Result<Option<String>, Box<dyn Error>> {
    let mut rdr = csv::Reader::from_path("./stops.txt")?;
    for result in rdr.records() {
        // The iterator yields Result<StringRecord, Error>, so we check the
        // error here.
        let record = result?;

        if record.get(1).unwrap() == stop_code {
            return Ok(Some(String::from(record.get(0).unwrap())));
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
    let stop_id_result = lookup_stop_code(&args.stop)?;
    if stop_id_result.is_none() {
        println!("Stop code {} not found", args.stop);
        return Ok(());
    }
    let arg_stop_id = stop_id_result.unwrap();

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
                && route_id == &args.route
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
