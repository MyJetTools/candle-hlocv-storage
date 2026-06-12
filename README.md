# candle-hlocv-storage

File-based HLOCV candle storage with O(1) offset access. Candles live in a flat binary file as year blocks; no database — a range read is one `seek` + a sequential `read`.

## Format

The component is given a base path (`open("/data/candles/binance")`) and appends the extensions itself:

| File | Contents |
|---|---|
| `<base>.index` | YAML index: an array of `{instrument_id, candle_type, year, offset}` — the offset of a year block's start in `.data`. Small, updated only when a new block is allocated (essentially once a year per instrument/timeframe) |
| `<base>.data` | Binary year blocks of fixed size |

The index is kept entirely in memory:

- on `open()` the `.index` file is read once, deserialized from YAML and lives on as a `Vec` + hash lookup;
- every index access during candle reads/writes is an in-memory lookup — the file is never re-read;
- on every update (allocation of a new block) the whole index is flushed from memory to disk — atomically, via `.tmp` → fsync → rename.

### Candle record — 40 bytes, little-endian

```
open f64 | high f64 | low f64 | close f64 | volume u64
```

Volume is an integer in minimal units (lot step) — a consumer-side convention, the component does not interpret it. An all-zero slot means "no candle" → `Option::None` (a real candle always has open > 0).

### Year blocks and slots

A block is always sized for a leap year — the size does not depend on the year, so a block offset is computed without any calendar:

| Type | Slots per year | Block size |
|---|---|---|
| Minute | 527,040 (366×1440) | ~20.1 MiB |
| Hour | 8,784 (366×24) | ~343 KiB |
| Day | 366 | 14,640 B |

A slot is the sequential period number since January 1 UTC: `01.Jan 00:00 → 0`, `01.Jan 00:01 → 1`; hour: `01.Jan 00h → 0`; day: `January 1 → 0`. Candle offset = `block_offset + slot × 40`. In a non-leap year the tail of the block (1440 minute slots) is simply unused; blocks are allocated via `set_len` (sparse), so unwritten tails take no disk space.

Month candles are **not stored** — they are aggregated on the fly from day candles at read time.

## API

Keys are `IntervalKey<MinuteKey/HourKey/DayKey/MonthKey>` from rust-extensions (`YYYYMMDDHHMM` / `YYYYMMDDHH` / `YYYYMMDD` / `YYYYMM`). Everything is async, state is behind a `tokio::sync::Mutex` — index and data change transactionally.

```rust
use candle_hlocv_storage::{CandleHLOCWriter, CandleModel};

let writer = CandleHLOCWriter::open("/data/candles/binance").await?;

// write (three typed methods: minute / hour / day)
writer.write_minute_candle("BTCUSDT", 2026, key, candle).await?;

// single read: all zeros -> None
let candle: Option<CandleModel> =
    writer.read_minute_candle("BTCUSDT", 2026, key).await?;

// range: one read per year, empty slots filtered out, crosses year boundaries
let history: Vec<(IntervalKey<MinuteKey>, CandleModel)> =
    writer.read_minute_candles("BTCUSDT", from, to).await?;

// month — on-the-fly aggregation of day candles (open of the first day,
// close of the last, max high / min low, volume sum)
let month = writer.read_month_candle("BTCUSDT", 2026, month_key).await?;
let months = writer.read_month_candles("BTCUSDT", from_month, to_month).await?;
```

The component is a pure writer/reader: M1→H1→D1 roll-up is done by the caller — ready-made candles of each timeframe come in.

## Modules

- `minute` / `hour` / `day` — block constants and pure slot math (`year_of`, `slot_of`, `key_from_slot`, `last_slot_of_year`), no I/O, covered by unit tests (leap-year shifts, roundtrip over every slot of a year).
- `month` — aggregation helpers (day-slot range of a month, fold).
- `calendar` — internal day-of-year ↔ month/day arithmetic, leap-year rule including centuries.

## Crash guarantees

- The index is saved atomically: `<base>.index.tmp` → fsync → rename.
- When allocating a block, `.data` is extended and fsynced first, then the index is saved. A crash in between leaves a zero tail that the next allocation reuses: `next_offset` is derived from the index, not from the file length.

## Tests

```bash
cargo test
```
