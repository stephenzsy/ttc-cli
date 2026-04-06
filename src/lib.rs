use prost::Message;
use reqwest::{self};
use std::error::Error;

pub mod transit_realtime {
    include!("protos/transit_realtime.rs");
}
pub mod db;
use crate::transit_realtime::FeedMessage;

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
    db: db::GtfsDb,
    feed_url: String,
}

pub struct ArrivalTime {
    pub rt_value: Option<i64>,
    pub scheduled: Option<String>,
}

pub struct TripAtStop {
    pub trip_id: Option<String>,
    pub vehicle_id: Option<String>,
    pub arrival_times: Vec<ArrivalTime>,
}

pub struct NextBusResult {
    pub timestamp: Option<u64>,
    pub route_id: String,
    pub stop_name: String,
    pub trips: Vec<TripAtStop>,
}

const DEFAULT_FEED_URL: &str = "https://gtfsrt.ttc.ca/trips/update?format=binary";

impl TTCRealTime {
    pub fn new(data_dir: &Option<String>) -> Result<Self, Box<dyn Error>> {
        let data_dir_path = db::resolve_data_dir(data_dir);
        let gtfs_db = db::GtfsDb::new(&data_dir_path.as_path(), false)?;

        Ok(Self {
            db: gtfs_db,
            feed_url: String::from(DEFAULT_FEED_URL),
        })
    }

    pub async fn fetch_feed(&self) -> Result<transit_realtime::FeedMessage, Box<dyn Error>> {
        let feed_raw = reqwest::get(&self.feed_url).await?.bytes().await?;
        let message = transit_realtime::FeedMessage::decode(feed_raw)?;
        Ok(message)
    }

    pub fn next_bus(
        &self,
        feed: &FeedMessage,
        route_id: &String,
        stop_code: &String,
    ) -> Result<NextBusResult, Box<dyn Error>> {
        let stop = self
            .db
            .get_stop_by_code(stop_code)
            .map_err(|_| format!("Stop code {} not found", stop_code))?;

        let trips = feed
            .entity
            .iter()
            .filter_map(|entity| {
                if !entity.is_deleted()
                    && let Some(trip_update) = &entity.trip_update
                    && let Some(trip_route_id) = &trip_update.trip.route_id
                    && trip_route_id == route_id
                {
                    let trip_id = &trip_update.trip.trip_id;
                    let arrival_times = trip_update
                        .stop_time_update
                        .iter()
                        .filter_map(|stop_time_update| {
                            if let Some(trip_stop_id) = &stop_time_update.stop_id
                                && trip_stop_id == &stop.stop_id
                            {
                                let mut scheduled: Option<String> = None;
                                if let Some(trip_id) = trip_id {
                                    let stop_time = self
                                        .db
                                        .get_scheduled_arrival(
                                            &trip_id,
                                            &stop_time_update.stop_sequence(),
                                        )
                                        .ok()?;
                                    if let Some(stop_time) = stop_time {
                                        scheduled = stop_time.arrival_time;
                                    }
                                }
                                Some(ArrivalTime {
                                    rt_value: stop_time_update.arrival?.time,
                                    scheduled: scheduled,
                                })
                            } else {
                                None
                            }
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

        Ok(NextBusResult {
            timestamp: feed.header.timestamp,
            route_id: route_id.clone(),
            stop_name: stop.stop_name.clone(),
            trips,
        })
    }
}
