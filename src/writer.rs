use std::io::SeekFrom;
use std::path::{Path, PathBuf};

use rust_extensions::date_time::{DayKey, HourKey, IntervalKey, MinuteKey, MonthKey};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio::sync::Mutex;

use crate::candle_model::{CandleModel, CANDLE_SIZE};
use crate::index::{CandleType, FileIndex, IndexEntry};
use crate::{day, hour, minute, month};

/// File-backed HLOCV candle storage: `<base>.index` (YAML) + `<base>.data` (binary
/// year blocks, every block sized for a leap year). Pure writer/reader — no
/// timeframe aggregation inside; an all-zero slot reads back as `None`.
pub struct CandleHLOCWriter {
    inner: Mutex<Inner>,
}

struct Inner {
    index: FileIndex,
    data: tokio::fs::File,
    index_path: PathBuf,
}

impl CandleHLOCWriter {
    /// `base_path` is the file name without extension — `.index` and `.data`
    /// are appended by the component.
    pub async fn open(base_path: impl AsRef<Path>) -> Result<Self, std::io::Error> {
        let base_path = base_path.as_ref();
        let index_path = append_extension(base_path, ".index");
        let data_path = append_extension(base_path, ".data");

        let index = FileIndex::load(&index_path).await?;
        let data = tokio::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&data_path)
            .await?;

        Ok(Self {
            inner: Mutex::new(Inner {
                index,
                data,
                index_path,
            }),
        })
    }

    /// Returns the list of all instruments that have at least one block in the index.
    pub async fn get_all_instruments(&self) -> Vec<String> {
        let inner = self.inner.lock().await;
        inner.index.get_all_instruments()
    }

    // --- Minute ---

    pub async fn write_minute_candle(
        &self,
        instrument_id: &str,
        year: u16,
        interval_key: IntervalKey<MinuteKey>,
        candle: CandleModel,
    ) -> Result<(), std::io::Error> {
        debug_assert_eq!(year, minute::year_of(interval_key));
        self.write_candle(
            instrument_id,
            CandleType::Minute,
            year,
            minute::slot_of(interval_key),
            candle,
        )
        .await
    }

    pub async fn read_minute_candle(
        &self,
        instrument_id: &str,
        year: u16,
        interval_key: IntervalKey<MinuteKey>,
    ) -> Result<Option<CandleModel>, std::io::Error> {
        debug_assert_eq!(year, minute::year_of(interval_key));
        self.read_candle(
            instrument_id,
            CandleType::Minute,
            year,
            minute::slot_of(interval_key),
        )
        .await
    }

    pub async fn read_minute_candles(
        &self,
        instrument_id: &str,
        from: IntervalKey<MinuteKey>,
        to: IntervalKey<MinuteKey>,
    ) -> Result<Vec<(IntervalKey<MinuteKey>, CandleModel)>, std::io::Error> {
        let mut result = Vec::new();
        for year in minute::year_of(from)..=minute::year_of(to) {
            let slot_from = if year == minute::year_of(from) {
                minute::slot_of(from)
            } else {
                0
            };
            let slot_to = if year == minute::year_of(to) {
                minute::slot_of(to)
            } else {
                minute::last_slot_of_year(year)
            };
            if slot_from > slot_to {
                continue;
            }
            for (slot, candle) in self
                .read_slots(instrument_id, CandleType::Minute, year, slot_from, slot_to)
                .await?
            {
                result.push((minute::key_from_slot(year, slot), candle));
            }
        }
        Ok(result)
    }

    // --- Hour ---

    pub async fn write_hour_candle(
        &self,
        instrument_id: &str,
        year: u16,
        interval_key: IntervalKey<HourKey>,
        candle: CandleModel,
    ) -> Result<(), std::io::Error> {
        debug_assert_eq!(year, hour::year_of(interval_key));
        self.write_candle(
            instrument_id,
            CandleType::Hour,
            year,
            hour::slot_of(interval_key),
            candle,
        )
        .await
    }

    pub async fn read_hour_candle(
        &self,
        instrument_id: &str,
        year: u16,
        interval_key: IntervalKey<HourKey>,
    ) -> Result<Option<CandleModel>, std::io::Error> {
        debug_assert_eq!(year, hour::year_of(interval_key));
        self.read_candle(
            instrument_id,
            CandleType::Hour,
            year,
            hour::slot_of(interval_key),
        )
        .await
    }

    pub async fn read_hour_candles(
        &self,
        instrument_id: &str,
        from: IntervalKey<HourKey>,
        to: IntervalKey<HourKey>,
    ) -> Result<Vec<(IntervalKey<HourKey>, CandleModel)>, std::io::Error> {
        let mut result = Vec::new();
        for year in hour::year_of(from)..=hour::year_of(to) {
            let slot_from = if year == hour::year_of(from) {
                hour::slot_of(from)
            } else {
                0
            };
            let slot_to = if year == hour::year_of(to) {
                hour::slot_of(to)
            } else {
                hour::last_slot_of_year(year)
            };
            if slot_from > slot_to {
                continue;
            }
            for (slot, candle) in self
                .read_slots(instrument_id, CandleType::Hour, year, slot_from, slot_to)
                .await?
            {
                result.push((hour::key_from_slot(year, slot), candle));
            }
        }
        Ok(result)
    }

    // --- Day ---

    pub async fn write_day_candle(
        &self,
        instrument_id: &str,
        year: u16,
        interval_key: IntervalKey<DayKey>,
        candle: CandleModel,
    ) -> Result<(), std::io::Error> {
        debug_assert_eq!(year, day::year_of(interval_key));
        self.write_candle(
            instrument_id,
            CandleType::Day,
            year,
            day::slot_of(interval_key),
            candle,
        )
        .await
    }

    pub async fn read_day_candle(
        &self,
        instrument_id: &str,
        year: u16,
        interval_key: IntervalKey<DayKey>,
    ) -> Result<Option<CandleModel>, std::io::Error> {
        debug_assert_eq!(year, day::year_of(interval_key));
        self.read_candle(
            instrument_id,
            CandleType::Day,
            year,
            day::slot_of(interval_key),
        )
        .await
    }

    pub async fn read_day_candles(
        &self,
        instrument_id: &str,
        from: IntervalKey<DayKey>,
        to: IntervalKey<DayKey>,
    ) -> Result<Vec<(IntervalKey<DayKey>, CandleModel)>, std::io::Error> {
        let mut result = Vec::new();
        for year in day::year_of(from)..=day::year_of(to) {
            let slot_from = if year == day::year_of(from) {
                day::slot_of(from)
            } else {
                0
            };
            let slot_to = if year == day::year_of(to) {
                day::slot_of(to)
            } else {
                day::last_slot_of_year(year)
            };
            if slot_from > slot_to {
                continue;
            }
            for (slot, candle) in self
                .read_slots(instrument_id, CandleType::Day, year, slot_from, slot_to)
                .await?
            {
                result.push((day::key_from_slot(year, slot), candle));
            }
        }
        Ok(result)
    }

    // --- Month (not stored — aggregated from day candles on the fly) ---

    pub async fn read_month_candle(
        &self,
        instrument_id: &str,
        year: u16,
        interval_key: IntervalKey<MonthKey>,
    ) -> Result<Option<CandleModel>, std::io::Error> {
        debug_assert_eq!(year, month::year_of(interval_key));
        let (slot_from, slot_to) = month::day_slot_range(year, month::month_of(interval_key));
        let days = self
            .read_slots(instrument_id, CandleType::Day, year, slot_from, slot_to)
            .await?;
        let days: Vec<CandleModel> = days.into_iter().map(|(_, candle)| candle).collect();
        Ok(month::aggregate(&days))
    }

    pub async fn read_month_candles(
        &self,
        instrument_id: &str,
        from: IntervalKey<MonthKey>,
        to: IntervalKey<MonthKey>,
    ) -> Result<Vec<(IntervalKey<MonthKey>, CandleModel)>, std::io::Error> {
        let mut result = Vec::new();
        let (mut year, mut month_no) = (month::year_of(from), month::month_of(from));
        let until = (month::year_of(to), month::month_of(to));

        while (year, month_no) <= until {
            let key = month::key_from(year, month_no);
            if let Some(candle) = self.read_month_candle(instrument_id, year, key).await? {
                result.push((key, candle));
            }
            (year, month_no) = month::next_month(year, month_no);
        }
        Ok(result)
    }

    // --- Index ---

    /// Snapshot of the index: which (instrument_id, candle_type, year) blocks
    /// exist. Lets the consumer enumerate stored instruments and year bounds
    /// without reading the data file.
    pub async fn get_index(&self) -> Vec<IndexEntry> {
        self.inner.lock().await.index.items().to_vec()
    }

    // --- Erase (zero out slot ranges; a zeroed slot reads back as `None`) ---

    pub async fn erase_minute_candles(
        &self,
        instrument_id: &str,
        from: IntervalKey<MinuteKey>,
        to: IntervalKey<MinuteKey>,
    ) -> Result<u64, std::io::Error> {
        let mut erased = 0;
        for year in minute::year_of(from)..=minute::year_of(to) {
            let slot_from = if year == minute::year_of(from) {
                minute::slot_of(from)
            } else {
                0
            };
            let slot_to = if year == minute::year_of(to) {
                minute::slot_of(to)
            } else {
                minute::last_slot_of_year(year)
            };
            if slot_from > slot_to {
                continue;
            }
            erased += self
                .erase_slots(instrument_id, CandleType::Minute, year, slot_from, slot_to)
                .await?;
        }
        Ok(erased)
    }

    pub async fn erase_hour_candles(
        &self,
        instrument_id: &str,
        from: IntervalKey<HourKey>,
        to: IntervalKey<HourKey>,
    ) -> Result<u64, std::io::Error> {
        let mut erased = 0;
        for year in hour::year_of(from)..=hour::year_of(to) {
            let slot_from = if year == hour::year_of(from) {
                hour::slot_of(from)
            } else {
                0
            };
            let slot_to = if year == hour::year_of(to) {
                hour::slot_of(to)
            } else {
                hour::last_slot_of_year(year)
            };
            if slot_from > slot_to {
                continue;
            }
            erased += self
                .erase_slots(instrument_id, CandleType::Hour, year, slot_from, slot_to)
                .await?;
        }
        Ok(erased)
    }

    pub async fn erase_day_candles(
        &self,
        instrument_id: &str,
        from: IntervalKey<DayKey>,
        to: IntervalKey<DayKey>,
    ) -> Result<u64, std::io::Error> {
        let mut erased = 0;
        for year in day::year_of(from)..=day::year_of(to) {
            let slot_from = if year == day::year_of(from) {
                day::slot_of(from)
            } else {
                0
            };
            let slot_to = if year == day::year_of(to) {
                day::slot_of(to)
            } else {
                day::last_slot_of_year(year)
            };
            if slot_from > slot_to {
                continue;
            }
            erased += self
                .erase_slots(instrument_id, CandleType::Day, year, slot_from, slot_to)
                .await?;
        }
        Ok(erased)
    }

    // --- Shared I/O ---

    async fn write_candle(
        &self,
        instrument_id: &str,
        candle_type: CandleType,
        year: u16,
        slot: u32,
        candle: CandleModel,
    ) -> Result<(), std::io::Error> {
        let mut inner = self.inner.lock().await;

        let block_offset = match inner.index.get(instrument_id, candle_type, year) {
            Some(offset) => offset,
            None => inner.allocate_block(instrument_id, candle_type, year).await?,
        };

        let offset = block_offset + slot as u64 * CANDLE_SIZE as u64;
        inner.data.seek(SeekFrom::Start(offset)).await?;
        inner.data.write_all(&candle.to_bytes()).await
    }

    async fn read_candle(
        &self,
        instrument_id: &str,
        candle_type: CandleType,
        year: u16,
        slot: u32,
    ) -> Result<Option<CandleModel>, std::io::Error> {
        let mut inner = self.inner.lock().await;

        let Some(block_offset) = inner.index.get(instrument_id, candle_type, year) else {
            return Ok(None);
        };

        let offset = block_offset + slot as u64 * CANDLE_SIZE as u64;
        inner.data.seek(SeekFrom::Start(offset)).await?;
        let mut buf = [0u8; CANDLE_SIZE];
        inner.data.read_exact(&mut buf).await?;

        if CandleModel::is_empty_slot(&buf) {
            Ok(None)
        } else {
            Ok(Some(CandleModel::from_bytes(&buf)))
        }
    }

    /// Reads slots `[slot_from..=slot_to]` of one year block as a single
    /// contiguous chunk; empty slots are filtered out.
    async fn read_slots(
        &self,
        instrument_id: &str,
        candle_type: CandleType,
        year: u16,
        slot_from: u32,
        slot_to: u32,
    ) -> Result<Vec<(u32, CandleModel)>, std::io::Error> {
        let mut inner = self.inner.lock().await;

        let Some(block_offset) = inner.index.get(instrument_id, candle_type, year) else {
            return Ok(Vec::new());
        };

        let count = (slot_to - slot_from + 1) as usize;
        let mut buf = vec![0u8; count * CANDLE_SIZE];
        inner
            .data
            .seek(SeekFrom::Start(
                block_offset + slot_from as u64 * CANDLE_SIZE as u64,
            ))
            .await?;
        inner.data.read_exact(&mut buf).await?;

        let mut result = Vec::new();
        for (index, chunk) in buf.chunks_exact(CANDLE_SIZE).enumerate() {
            let chunk: &[u8; CANDLE_SIZE] = chunk.try_into().unwrap();
            if !CandleModel::is_empty_slot(chunk) {
                result.push((slot_from + index as u32, CandleModel::from_bytes(chunk)));
            }
        }
        Ok(result)
    }

    /// Zeroes slots `[slot_from..=slot_to]` of one year block; returns how many
    /// non-empty slots were actually erased (the range is read first to count).
    async fn erase_slots(
        &self,
        instrument_id: &str,
        candle_type: CandleType,
        year: u16,
        slot_from: u32,
        slot_to: u32,
    ) -> Result<u64, std::io::Error> {
        let mut inner = self.inner.lock().await;

        let Some(block_offset) = inner.index.get(instrument_id, candle_type, year) else {
            return Ok(0);
        };

        let count = (slot_to - slot_from + 1) as usize;
        let range_offset = block_offset + slot_from as u64 * CANDLE_SIZE as u64;

        let mut buf = vec![0u8; count * CANDLE_SIZE];
        inner.data.seek(SeekFrom::Start(range_offset)).await?;
        inner.data.read_exact(&mut buf).await?;

        let erased = buf
            .chunks_exact(CANDLE_SIZE)
            .filter(|chunk| {
                let chunk: &[u8; CANDLE_SIZE] = (*chunk).try_into().unwrap();
                !CandleModel::is_empty_slot(chunk)
            })
            .count() as u64;
        if erased == 0 {
            return Ok(0);
        }

        buf.fill(0);
        inner.data.seek(SeekFrom::Start(range_offset)).await?;
        inner.data.write_all(&buf).await?;
        Ok(erased)
    }
}

impl Inner {
    /// Allocates a year block at the end of the used area: extends the data file
    /// (sparse zeros), fsyncs it, then persists the index atomically. Data is
    /// extended before the index is saved so a crash in between leaves only an
    /// orphaned zero tail, which `FileIndex::next_offset` reuses.
    async fn allocate_block(
        &mut self,
        instrument_id: &str,
        candle_type: CandleType,
        year: u16,
    ) -> Result<u64, std::io::Error> {
        let offset = self.index.next_offset();
        let target_len = offset + candle_type.block_size();

        if target_len > self.data.metadata().await?.len() {
            self.data.set_len(target_len).await?;
        }
        self.data.sync_all().await?;

        self.index.add(IndexEntry {
            instrument_id: instrument_id.to_string(),
            candle_type,
            year,
            offset,
        });
        if let Err(err) = self.index.save(&self.index_path).await {
            self.index.remove_last();
            return Err(err);
        }

        Ok(offset)
    }
}

fn append_extension(base_path: &Path, extension: &str) -> PathBuf {
    let mut result = base_path.as_os_str().to_owned();
    result.push(extension);
    result.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candle(price: f64, volume: u64) -> CandleModel {
        CandleModel {
            open: price,
            high: price + 1.0,
            low: price - 1.0,
            close: price + 0.5,
            volume,
        }
    }

    #[tokio::test]
    async fn write_and_read_all_timeframes() {
        let dir = tempfile::tempdir().unwrap();
        let writer = CandleHLOCWriter::open(dir.path().join("candles")).await.unwrap();

        let minute_key = IntervalKey::<MinuteKey>::from_i64(202606121234);
        let hour_key = IntervalKey::<HourKey>::from_i64(2026061212);
        let day_key = IntervalKey::<DayKey>::from_i64(20260612);

        writer
            .write_minute_candle("BTCUSDT", 2026, minute_key, candle(104325.57, 100))
            .await
            .unwrap();
        writer
            .write_hour_candle("BTCUSDT", 2026, hour_key, candle(104000.0, 6000))
            .await
            .unwrap();
        writer
            .write_day_candle("BTCUSDT", 2026, day_key, candle(103000.0, 144000))
            .await
            .unwrap();

        assert_eq!(
            writer
                .read_minute_candle("BTCUSDT", 2026, minute_key)
                .await
                .unwrap(),
            Some(candle(104325.57, 100))
        );
        assert_eq!(
            writer
                .read_hour_candle("BTCUSDT", 2026, hour_key)
                .await
                .unwrap(),
            Some(candle(104000.0, 6000))
        );
        assert_eq!(
            writer
                .read_day_candle("BTCUSDT", 2026, day_key)
                .await
                .unwrap(),
            Some(candle(103000.0, 144000))
        );
    }

    #[tokio::test]
    async fn empty_slot_and_missing_year_read_as_none() {
        let dir = tempfile::tempdir().unwrap();
        let writer = CandleHLOCWriter::open(dir.path().join("candles")).await.unwrap();

        writer
            .write_minute_candle(
                "BTCUSDT",
                2026,
                IntervalKey::from_i64(202606121234),
                candle(104325.57, 100),
            )
            .await
            .unwrap();

        // same year, neighbour slot never written
        assert_eq!(
            writer
                .read_minute_candle("BTCUSDT", 2026, IntervalKey::from_i64(202606121235))
                .await
                .unwrap(),
            None
        );
        // year that has no block in the index
        assert_eq!(
            writer
                .read_minute_candle("BTCUSDT", 2025, IntervalKey::from_i64(202506121234))
                .await
                .unwrap(),
            None
        );
        // instrument that has no blocks at all
        assert_eq!(
            writer
                .read_minute_candle("ETHUSDT", 2026, IntervalKey::from_i64(202606121234))
                .await
                .unwrap(),
            None
        );
    }

    #[tokio::test]
    async fn range_read_filters_empty_slots() {
        let dir = tempfile::tempdir().unwrap();
        let writer = CandleHLOCWriter::open(dir.path().join("candles")).await.unwrap();

        writer
            .write_minute_candle(
                "BTCUSDT",
                2026,
                IntervalKey::from_i64(202606121200),
                candle(1.0, 1),
            )
            .await
            .unwrap();
        writer
            .write_minute_candle(
                "BTCUSDT",
                2026,
                IntervalKey::from_i64(202606121203),
                candle(2.0, 2),
            )
            .await
            .unwrap();

        let result = writer
            .read_minute_candles(
                "BTCUSDT",
                IntervalKey::from_i64(202606121200),
                IntervalKey::from_i64(202606121210),
            )
            .await
            .unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].0.to_i64(), 202606121200);
        assert_eq!(result[0].1, candle(1.0, 1));
        assert_eq!(result[1].0.to_i64(), 202606121203);
        assert_eq!(result[1].1, candle(2.0, 2));
    }

    #[tokio::test]
    async fn range_read_crosses_year_boundary() {
        let dir = tempfile::tempdir().unwrap();
        let writer = CandleHLOCWriter::open(dir.path().join("candles")).await.unwrap();

        writer
            .write_minute_candle(
                "BTCUSDT",
                2025,
                IntervalKey::from_i64(202512312359),
                candle(1.0, 1),
            )
            .await
            .unwrap();
        writer
            .write_minute_candle(
                "BTCUSDT",
                2026,
                IntervalKey::from_i64(202601010000),
                candle(2.0, 2),
            )
            .await
            .unwrap();

        let result = writer
            .read_minute_candles(
                "BTCUSDT",
                IntervalKey::from_i64(202512312300),
                IntervalKey::from_i64(202601010100),
            )
            .await
            .unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].0.to_i64(), 202512312359);
        assert_eq!(result[1].0.to_i64(), 202601010000);
    }

    #[tokio::test]
    async fn month_candle_aggregates_days() {
        let dir = tempfile::tempdir().unwrap();
        let writer = CandleHLOCWriter::open(dir.path().join("candles")).await.unwrap();

        writer
            .write_day_candle(
                "BTCUSDT",
                2026,
                IntervalKey::from_i64(20260601),
                CandleModel { open: 10.0, high: 12.0, low: 9.0, close: 11.0, volume: 100 },
            )
            .await
            .unwrap();
        writer
            .write_day_candle(
                "BTCUSDT",
                2026,
                IntervalKey::from_i64(20260615),
                CandleModel { open: 11.0, high: 15.0, low: 10.5, close: 14.0, volume: 200 },
            )
            .await
            .unwrap();
        writer
            .write_day_candle(
                "BTCUSDT",
                2026,
                IntervalKey::from_i64(20260630),
                CandleModel { open: 14.0, high: 14.5, low: 8.0, close: 9.5, volume: 300 },
            )
            .await
            .unwrap();
        // neighbour month must not leak into June
        writer
            .write_day_candle(
                "BTCUSDT",
                2026,
                IntervalKey::from_i64(20260701),
                CandleModel { open: 100.0, high: 200.0, low: 1.0, close: 150.0, volume: 999 },
            )
            .await
            .unwrap();

        assert_eq!(
            writer
                .read_month_candle("BTCUSDT", 2026, IntervalKey::from_i64(202606))
                .await
                .unwrap(),
            Some(CandleModel { open: 10.0, high: 15.0, low: 8.0, close: 9.5, volume: 600 })
        );
        // month with no day candles at all
        assert_eq!(
            writer
                .read_month_candle("BTCUSDT", 2026, IntervalKey::from_i64(202605))
                .await
                .unwrap(),
            None
        );
    }

    #[tokio::test]
    async fn month_range_crosses_year_boundary_and_skips_empty_months() {
        let dir = tempfile::tempdir().unwrap();
        let writer = CandleHLOCWriter::open(dir.path().join("candles")).await.unwrap();

        writer
            .write_day_candle(
                "BTCUSDT",
                2025,
                IntervalKey::from_i64(20251215),
                candle(1.0, 1),
            )
            .await
            .unwrap();
        writer
            .write_day_candle(
                "BTCUSDT",
                2026,
                IntervalKey::from_i64(20260210),
                candle(2.0, 2),
            )
            .await
            .unwrap();

        let result = writer
            .read_month_candles(
                "BTCUSDT",
                IntervalKey::from_i64(202511),
                IntervalKey::from_i64(202603),
            )
            .await
            .unwrap();

        // Nov-2025, Jan-2026 and Mar-2026 are empty and skipped
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].0.to_i64(), 202512);
        assert_eq!(result[0].1, candle(1.0, 1));
        assert_eq!(result[1].0.to_i64(), 202602);
        assert_eq!(result[1].1, candle(2.0, 2));
    }

    #[tokio::test]
    async fn get_index_enumerates_blocks() {
        let dir = tempfile::tempdir().unwrap();
        let writer = CandleHLOCWriter::open(dir.path().join("candles")).await.unwrap();

        assert!(writer.get_index().await.is_empty());

        writer
            .write_minute_candle(
                "BTCUSDT",
                2026,
                IntervalKey::from_i64(202606121234),
                candle(1.0, 1),
            )
            .await
            .unwrap();
        writer
            .write_day_candle("ETHUSDT", 2025, IntervalKey::from_i64(20250612), candle(2.0, 2))
            .await
            .unwrap();

        let index = writer.get_index().await;
        assert_eq!(index.len(), 2);
        assert_eq!(index[0].instrument_id, "BTCUSDT");
        assert_eq!(index[0].candle_type, CandleType::Minute);
        assert_eq!(index[0].year, 2026);
        assert_eq!(index[1].instrument_id, "ETHUSDT");
        assert_eq!(index[1].candle_type, CandleType::Day);
        assert_eq!(index[1].year, 2025);
    }

    #[tokio::test]
    async fn erase_zeroes_range_and_keeps_neighbours() {
        let dir = tempfile::tempdir().unwrap();
        let writer = CandleHLOCWriter::open(dir.path().join("candles")).await.unwrap();

        for (i, key) in [202606121200i64, 202606121201, 202606121202, 202606121203]
            .into_iter()
            .enumerate()
        {
            writer
                .write_minute_candle(
                    "BTCUSDT",
                    2026,
                    IntervalKey::from_i64(key),
                    candle(1.0 + i as f64, 1),
                )
                .await
                .unwrap();
        }

        let erased = writer
            .erase_minute_candles(
                "BTCUSDT",
                IntervalKey::from_i64(202606121201),
                IntervalKey::from_i64(202606121202),
            )
            .await
            .unwrap();
        assert_eq!(erased, 2);

        let left = writer
            .read_minute_candles(
                "BTCUSDT",
                IntervalKey::from_i64(202606121200),
                IntervalKey::from_i64(202606121203),
            )
            .await
            .unwrap();
        assert_eq!(left.len(), 2);
        assert_eq!(left[0].0.to_i64(), 202606121200);
        assert_eq!(left[1].0.to_i64(), 202606121203);

        // erasing an already-empty range (and a missing instrument) is a no-op
        assert_eq!(
            writer
                .erase_minute_candles(
                    "BTCUSDT",
                    IntervalKey::from_i64(202606121201),
                    IntervalKey::from_i64(202606121202),
                )
                .await
                .unwrap(),
            0
        );
        assert_eq!(
            writer
                .erase_minute_candles(
                    "ETHUSDT",
                    IntervalKey::from_i64(202606121200),
                    IntervalKey::from_i64(202606121203),
                )
                .await
                .unwrap(),
            0
        );
    }

    #[tokio::test]
    async fn erase_crosses_year_boundary() {
        let dir = tempfile::tempdir().unwrap();
        let writer = CandleHLOCWriter::open(dir.path().join("candles")).await.unwrap();

        writer
            .write_day_candle("BTCUSDT", 2025, IntervalKey::from_i64(20251230), candle(1.0, 1))
            .await
            .unwrap();
        writer
            .write_day_candle("BTCUSDT", 2026, IntervalKey::from_i64(20260102), candle(2.0, 2))
            .await
            .unwrap();
        writer
            .write_day_candle("BTCUSDT", 2026, IntervalKey::from_i64(20260110), candle(3.0, 3))
            .await
            .unwrap();

        let erased = writer
            .erase_day_candles(
                "BTCUSDT",
                IntervalKey::from_i64(20251229),
                IntervalKey::from_i64(20260105),
            )
            .await
            .unwrap();
        assert_eq!(erased, 2);

        let left = writer
            .read_day_candles(
                "BTCUSDT",
                IntervalKey::from_i64(20251201),
                IntervalKey::from_i64(20260131),
            )
            .await
            .unwrap();
        assert_eq!(left.len(), 1);
        assert_eq!(left[0].0.to_i64(), 20260110);
    }

    #[tokio::test]
    async fn data_survives_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().join("candles");
        let key = IntervalKey::<MinuteKey>::from_i64(202606121234);

        {
            let writer = CandleHLOCWriter::open(&base).await.unwrap();
            writer
                .write_minute_candle("BTCUSDT", 2026, key, candle(104325.57, 100))
                .await
                .unwrap();
        }

        let writer = CandleHLOCWriter::open(&base).await.unwrap();
        assert_eq!(
            writer.read_minute_candle("BTCUSDT", 2026, key).await.unwrap(),
            Some(candle(104325.57, 100))
        );
    }

    #[tokio::test]
    async fn blocks_are_allocated_sequentially() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().join("candles");
        let writer = CandleHLOCWriter::open(&base).await.unwrap();

        writer
            .write_minute_candle(
                "BTCUSDT",
                2026,
                IntervalKey::from_i64(202606121234),
                candle(1.0, 1),
            )
            .await
            .unwrap();
        writer
            .write_hour_candle("BTCUSDT", 2026, IntervalKey::from_i64(2026061212), candle(2.0, 2))
            .await
            .unwrap();
        writer
            .write_day_candle("ETHUSDT", 2026, IntervalKey::from_i64(20260612), candle(3.0, 3))
            .await
            .unwrap();
        writer
            .write_minute_candle(
                "ETHUSDT",
                2025,
                IntervalKey::from_i64(202506121234),
                candle(4.0, 4),
            )
            .await
            .unwrap();

        // every candle still reads back from its own block
        assert_eq!(
            writer
                .read_minute_candle("BTCUSDT", 2026, IntervalKey::from_i64(202606121234))
                .await
                .unwrap(),
            Some(candle(1.0, 1))
        );
        assert_eq!(
            writer
                .read_hour_candle("BTCUSDT", 2026, IntervalKey::from_i64(2026061212))
                .await
                .unwrap(),
            Some(candle(2.0, 2))
        );
        assert_eq!(
            writer
                .read_day_candle("ETHUSDT", 2026, IntervalKey::from_i64(20260612))
                .await
                .unwrap(),
            Some(candle(3.0, 3))
        );
        assert_eq!(
            writer
                .read_minute_candle("ETHUSDT", 2025, IntervalKey::from_i64(202506121234))
                .await
                .unwrap(),
            Some(candle(4.0, 4))
        );

        // data file covers all four blocks exactly
        let data_len = tokio::fs::metadata(append_extension(&base, ".data"))
            .await
            .unwrap()
            .len();
        assert_eq!(
            data_len,
            minute::BLOCK_SIZE * 2 + hour::BLOCK_SIZE + day::BLOCK_SIZE
        );
    }
}
