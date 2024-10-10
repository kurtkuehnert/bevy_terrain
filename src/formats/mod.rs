use crate::math::TileCoordinate;
use anyhow::Result;
use ron::error::SpannedResult;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

mod tiff;

pub use crate::formats::tiff::TiffLoader;

#[derive(Serialize, Deserialize, Debug)]
pub struct TC {
    pub tiles: Vec<TileCoordinate>,
}

impl TC {
    pub fn load_file<P: AsRef<Path>>(path: P) -> SpannedResult<Self> {
        let encoded = fs::read_to_string(path)?;
        ron::from_str(&encoded)
    }

    pub fn save_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let encoded = ron::to_string(self)?;
        fs::write(path, encoded)?;
        Ok(())
    }
}
