use std::path::Path;

use ahash::AHashMap;

use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;

use crate::{day, hour, minute};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum CandleType {
    Minute,
    Hour,
    Day,
}

impl CandleType {
    pub fn block_size(&self) -> u64 {
        match self {
            CandleType::Minute => minute::BLOCK_SIZE,
            CandleType::Hour => hour::BLOCK_SIZE,
            CandleType::Day => day::BLOCK_SIZE,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct IndexEntry {
    pub instrument_id: String,
    pub candle_type: CandleType,
    pub year: u16,
    pub offset: u64,
}

#[derive(Serialize, Deserialize, Default)]
struct IndexFileContent {
    items: Vec<IndexEntry>,
}

#[derive(Default)]
pub struct FileIndex {
    items: Vec<IndexEntry>,
    lookup: AHashMap<String, AHashMap<(CandleType, u16), u64>>,
}

impl FileIndex {
    pub async fn load(path: &Path) -> Result<Self, std::io::Error> {
        let content = match tokio::fs::read(path).await {
            Ok(content) => content,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(err) => return Err(err),
        };

        let content: IndexFileContent = serde_yaml::from_slice(&content).map_err(|err| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("can not parse index file {}: {}", path.display(), err),
            )
        })?;

        let mut result = Self::default();
        for entry in content.items {
            result.insert_to_lookup(&entry);
            result.items.push(entry);
        }
        Ok(result)
    }

    /// Atomic: writes `<path>.tmp`, fsyncs, then renames over `<path>`.
    pub async fn save(&self, path: &Path) -> Result<(), std::io::Error> {
        let yaml = serde_yaml::to_string(&IndexFileContent {
            items: self.items.clone(),
        })
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err.to_string()))?;

        let mut tmp_path = path.as_os_str().to_owned();
        tmp_path.push(".tmp");

        let mut file = tokio::fs::File::create(&tmp_path).await?;
        file.write_all(yaml.as_bytes()).await?;
        file.sync_all().await?;
        drop(file);

        tokio::fs::rename(&tmp_path, path).await
    }

    pub fn items(&self) -> &[IndexEntry] {
        &self.items
    }

    pub fn get_all_instruments(&self) -> Vec<String> {
        self.lookup.keys().cloned().collect()
    }

    pub fn get(&self, instrument_id: &str, candle_type: CandleType, year: u16) -> Option<u64> {
        self.lookup
            .get(instrument_id)?
            .get(&(candle_type, year))
            .copied()
    }

    pub fn add(&mut self, entry: IndexEntry) {
        self.insert_to_lookup(&entry);
        self.items.push(entry);
    }

    pub fn remove_last(&mut self) {
        if let Some(entry) = self.items.pop() {
            if let Some(by_instrument) = self.lookup.get_mut(&entry.instrument_id) {
                by_instrument.remove(&(entry.candle_type, entry.year));
            }
        }
    }

    /// Next free offset is derived from the index, not from the data file length:
    /// a tail block orphaned by a crash between extending the data file and saving
    /// the index gets reused by the next allocation instead of being lost.
    pub fn next_offset(&self) -> u64 {
        self.items
            .iter()
            .map(|entry| entry.offset + entry.candle_type.block_size())
            .max()
            .unwrap_or(0)
    }

    fn insert_to_lookup(&mut self, entry: &IndexEntry) {
        self.lookup
            .entry(entry.instrument_id.clone())
            .or_default()
            .insert((entry.candle_type, entry.year), entry.offset);
    }
}
