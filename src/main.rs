use chrono::{DateTime, Local};
use clap::Parser;
use std::error::Error;

pub mod transit_realtime {
    include!("protos/transit_realtime.rs");
}

#[derive(clap::Parser, Debug)]
#[command(version)]
struct Args {
    #[arg(short, long)]
    route: String,

    #[arg(short, long)]
    stop: String,

    #[arg(long)]
    data_dir: Option<String>,
}

fn timestamp_to_local(timestamp: i64) -> Option<DateTime<Local>> {
    DateTime::from_timestamp(timestamp, 0).map(|dt| dt.with_timezone(&Local))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let session = ttc_cli::TTCRealTime::new(&args.data_dir)?;
    let feed = session.fetch_feed().await?;
    let next_bus = session.next_bus(&feed, &args.route, &args.stop)?;

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
                if let Some(ts) = arrival_time.rt_value {
                    if let Some(local_arrival) = timestamp_to_local(ts) {
                        println!("  Arrival Time: {}", local_arrival);
                    }
                } else {
                    println!("  Arrival Time: N/A (No Data)");
                }
                if let Some(scheduled) = arrival_time.scheduled {
                    println!("  Scheduled Arrival Time: {}", scheduled);
                }
            }
        }
    }
    Ok(())
}
