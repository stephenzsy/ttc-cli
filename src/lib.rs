use prost::Message;
use reqwest::{self};
use std::collections::hash_map::HashMap;
use std::error::Error;

pub mod transit_realtime {
    include!("protos/transit_realtime.rs");
}

#[derive(Debug, serde::Deserialize)]
pub struct StopRecord {
    pub stop_id: String,
    pub stop_name: String,
    pub stop_code: String,
}

pub struct Config {
    pub feed_url: String,
    pub stops_csv_path: String,
}

#[derive(Debug)]
pub struct StopTimeInfo {
    pub trip_id: String,
    pub stop_id: String,
    pub departure_time: String,
    pub arrival_time: String,
}

pub struct TTCRealTime {
    stops: HashMap<String, StopRecord>,
    feed_url: String,
    //    show_scheduled: bool,
}

pub struct TripAtStop {
    pub trip_id: Option<String>,
    pub vehicle_id: Option<String>,
    pub arrival_times: Vec<Option<i64>>,
}

pub struct NextBusResult {
    pub timestamp: Option<u64>,
    pub route_id: String,
    pub stop_name: String,
    pub trips: Vec<TripAtStop>,
}

const DEFAULT_FEED_URL: &str = "https://gtfsrt.ttc.ca/trips/update?format=binary";
const DEFAULT_STOPS_CSV_PATH: &str = "./stops.txt";

impl TTCRealTime {
    pub fn new(config: Option<Config>) -> Result<Self, Box<dyn Error>> {
        let config = config.unwrap_or_else(|| Config {
            feed_url: DEFAULT_FEED_URL.to_string(),
            stops_csv_path: DEFAULT_STOPS_CSV_PATH.to_string(),
        });
        let mut s = Self {
            stops: HashMap::new(),
            feed_url: config.feed_url,
        };
        load_stops(&mut s.stops, config.stops_csv_path)?;
        Ok(s)
    }

    pub async fn next_bus(
        &self,
        filter_route_id: String,
        stop_code: String,
    ) -> Result<NextBusResult, Box<dyn Error>> {
        if let Some(stop) = self.stops.get(&stop_code) {
            // fetch feed
            let feed_raw = reqwest::get(&self.feed_url).await?.bytes().await?;
            let message = transit_realtime::FeedMessage::decode(feed_raw)?;

            let trips = message
                .entity
                .iter()
                .filter_map(|entity| {
                    if !entity.is_deleted()
                        && let Some(trip_update) = &entity.trip_update
                        && let Some(route_id) = &trip_update.trip.route_id
                        && route_id == &filter_route_id
                    {
                        let trip_id = &trip_update.trip.trip_id;
                        let arrival_times = trip_update
                            .stop_time_update
                            .iter()
                            .filter_map(|stop_time_update| {
                                if let Some(stop_id) = &stop_time_update.stop_id
                                    && stop_id == &stop.stop_id
                                {
                                    return Some(stop_time_update.arrival?.time);
                                }
                                None
                            })
                            .collect::<Vec<_>>();
                        return Some(TripAtStop {
                            trip_id: trip_id.clone(),
                            vehicle_id: trip_update.vehicle.clone()?.id,
                            arrival_times,
                        });
                    }
                    None
                })
                .collect::<Vec<_>>();

            return Ok(NextBusResult {
                timestamp: message.header.timestamp,
                route_id: filter_route_id,
                stop_name: stop.stop_name.clone(),
                trips,
            });
        }
        Err(format!("Stop code {} not found", stop_code).into())
    }
}

fn load_stops(
    stops: &mut HashMap<String, StopRecord>,
    csv_path: String,
) -> Result<(), Box<dyn Error>> {
    let mut rdr = csv::Reader::from_path(csv_path)?;
    for result in rdr.deserialize() {
        // The iterator yields Result<StringRecord, Error>, so we check the
        // error here.
        let record: StopRecord = result?;
        stops.insert(record.stop_code.clone(), record);
    }
    Ok(())
}

/*
fn load_stop_times(
    stop_times: &mut HashMap<String, HashMap<String, StopTimeInfo>>,
    csv_path: String,
) -> Result<(), Box<dyn Error>> {
    let mut rdr = csv::Reader::from_path(csv_path)?;
    for result in rdr.records() {
        // The iterator yields Result<StringRecord, Error>, so we check the
        // error here.
        let record = result?;
        let stop_time = StopTimeInfo {
            trip_id: record.get(0).unwrap_or_default().to_string(),
            stop_id: record.get(3).unwrap_or_default().to_string(),
            arrival_time: record.get(1).unwrap_or_default().to_string(),
            departure_time: record.get(2).unwrap_or_default().to_string(),
        };
        stop_times
            .entry(stop_time.trip_id.clone())
            .or_insert_with(HashMap::new)
            .insert(stop_time.stop_id.clone(), stop_time);
    }
    Ok(())
}
*/
