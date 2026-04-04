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
