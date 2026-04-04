use rusqlite::{Connection, OpenFlags};
use std::{
    env,
    path::{Path, PathBuf},
};

pub fn resolve_data_dir(data_dir: &Option<String>) -> PathBuf {
    if let Some(data_dir) = data_dir {
        PathBuf::from(data_dir)
    } else {
        let parent: String = match env::var("HOME") {
            Ok(v) => v,
            Err(_) => String::from("."),
        };
        Path::new(&parent).join(".ttc")
    }
}

pub struct GtfsDb {
    conn: Connection,
}

#[derive(Debug, serde::Deserialize)]
pub struct Stop {
    pub stop_id: String,
    pub stop_name: String,
    pub stop_code: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct StopTime {
    pub trip_id: String,
    pub stop_sequence: String,
    pub stop_id: String,
    pub arrival_time: Option<String>,
}

impl GtfsDb {
    pub fn new(data_dir: &Path, writable: bool) -> Result<Self, Box<dyn std::error::Error>> {
        let db_path = data_dir.join("gfts.db");
        let conn: Connection = if writable {
            Connection::open(db_path)?
        } else {
            Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?
        };
        Ok(GtfsDb { conn })
    }

    pub fn clean(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.conn.execute("DROP TABLE IF EXISTS stops", ())?;
        self.conn.execute("DROP TABLE IF EXISTS stop_times", ())?;
        self.conn.execute(
            "CREATE TABLE stops (
            stop_id   TEXT NOT NULL PRIMARY KEY,
            stop_code TEXT NOT NULL,
            stop_name TEXT
        )",
            (), // empty list of parameters.
        )?;
        self.conn.execute(
            "CREATE UNIQUE INDEX index_stop_code ON stops (stop_code)",
            (), // empty list of parameters.
        )?;
        self.conn.execute(
            "CREATE TABLE stop_times (
            trip_id   TEXT    NOT NULL,
            stop_seq  INTEGER NOT NULL,
            stop_id   TEXT    NOT NULL,
            arrival   TEXT,
            PRIMARY KEY (trip_id, stop_seq)
        )",
            (), // empty list of parameters.
        )?;
        self.conn.execute(
            "CREATE INDEX index_stop_id ON stop_times (stop_id)",
            (), // empty list of parameters.
        )?;
        Ok(())
    }

    pub fn insert_stop(&self, stop: &Stop) -> Result<(), Box<dyn std::error::Error>> {
        self.conn.execute(
            "INSERT INTO stops (stop_id, stop_code, stop_name) VALUES (?1, ?2, ?3)",
            (&stop.stop_id, &stop.stop_code, &stop.stop_name),
        )?;
        Ok(())
    }

    pub fn delete_stop_times(&self, stop_id: &String) -> Result<(), Box<dyn std::error::Error>> {
        self.conn
            .execute("DELETE FROM stop_times WHERE stop_id = ?", (stop_id,))?;
        Ok(())
    }

    pub fn insert_stop_time(&self, stop_time: &StopTime) -> Result<(), Box<dyn std::error::Error>> {
        self.conn.execute(
            "INSERT INTO stop_times (trip_id, stop_seq, stop_id, arrival) VALUES (?1, ?2, ?3, ?4)",
            (
                &stop_time.trip_id,
                &stop_time.stop_sequence,
                &stop_time.stop_id,
                &stop_time.arrival_time,
            ),
        )?;
        Ok(())
    }

    pub fn get_stop_by_code(&self, stop_code: &String) -> Result<Stop, Box<dyn std::error::Error>> {
        let mut stmt = self
            .conn
            .prepare("SELECT stop_id, stop_code, stop_name FROM stops WHERE stop_code = ?")?;
        let stop = stmt.query_one([stop_code], |row| {
            Ok(Stop {
                stop_id: row.get(0)?,
                stop_code: row.get(1)?,
                stop_name: row.get(2)?,
            })
        })?;
        Ok(stop)
    }
}
