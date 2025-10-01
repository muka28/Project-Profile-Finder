use anyhow::Result;
use bincode;
use rstar::RTree;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;

use crate::domain::RoadGraph;
use crate::infrastructure::SpatialEdge;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct AppData {
    pub graph: RoadGraph,
    pub rtree: RTree<SpatialEdge>,
}

pub fn save_data(data: &AppData, path: &Path) -> Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    bincode::serialize_into(&mut writer, data)?;
    Ok(())
}

pub fn load_data(path: &Path) -> Result<AppData> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let data: AppData = bincode::deserialize_from(reader)?;
    Ok(data)
}