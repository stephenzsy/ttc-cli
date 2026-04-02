use prost::Message;
use reqwest::{self};
use std::collections::hash_map::HashMap;
use std::error::Error;

pub mod transit_realtime {
    include!("protos/transit_realtime.rs");
}

pub struct StopInfo {
    pub id: String,
    pub name: String,
    pub code: String,
}

pub struct Config {
    pub feed_url: String,
    pub stops_csv_path: String,
}

pub struct TTCRealTime {
    stops: HashMap<String, StopInfo>,
    feed_url: String,
}

pub struct Trip {
    pub vehicle_id: Option<String>,
    pub arrival_time: Option<i64>,
}

pub struct NextBusResult {
    pub timestamp: Option<u64>,
    pub route_id: String,
    pub stop_name: String,
    pub trips: Vec<Trip>,
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
                        let filtered_stoptime_update = trip_update
                            .stop_time_update
                            .iter()
                            .filter_map(|stop_time_update| {
                                if let Some(stop_id) = &stop_time_update.stop_id {
                                    if *stop_id == stop.id {
                                        return Some(stop_time_update);
                                    }
                                }
                                None
                            })
                            .collect::<Vec<_>>();
                        if filtered_stoptime_update.len() == 0 {
                            return None;
                        }
                        return Some(Trip {
                            vehicle_id: trip_update.vehicle.clone()?.id,
                            arrival_time: trip_update.stop_time_update[0].arrival?.time,
                        });
                    }
                    None
                })
                .collect::<Vec<_>>();

            return Ok(NextBusResult {
                timestamp: message.header.timestamp,
                route_id: filter_route_id,
                stop_name: stop.name.clone(),
                trips,
            });
        }
        Err(format!("Stop code {} not found", stop_code).into())
    }
}

fn load_stops(
    stops: &mut HashMap<String, StopInfo>,
    csv_path: String,
) -> Result<(), Box<dyn Error>> {
    let mut rdr = csv::Reader::from_path(csv_path)?;
    for result in rdr.records() {
        // The iterator yields Result<StringRecord, Error>, so we check the
        // error here.
        let record = result?;
        let stop = StopInfo {
            id: record.get(0).unwrap_or_default().to_string(),
            name: record.get(2).unwrap_or_default().to_string(),
            code: record.get(1).unwrap_or_default().to_string(),
        };
        stops.insert(stop.code.clone(), stop);
    }
    Ok(())
}
