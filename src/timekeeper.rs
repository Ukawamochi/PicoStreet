//! 単純な時刻管理（NTP同期後にUNIX時刻を推定）
use core::cell::Cell;
use embassy_time::Instant;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;

// NTP同期時のベース: (unix秒, Instant)
static TIMEBASE: Mutex<CriticalSectionRawMutex, Cell<Option<(u64, Instant)>>> =
    Mutex::new(Cell::new(None));

/// NTP同期したUNIX秒をセット（以後の now_unix() はこの値を基準に進む）
pub fn set_unix_time(unix: u64) {
    if let Ok(guard) = TIMEBASE.try_lock() {
        guard.set(Some((unix, Instant::now())));
    }
}

/// 現在のUNIX秒を返す（NTP未同期なら None）
pub fn now_unix() -> Option<u64> {
    if let Ok(guard) = TIMEBASE.try_lock() {
        if let Some((base, t0)) = guard.get() {
            let dt = Instant::now() - t0;
            return Some(base + dt.as_secs());
        }
    }
    None
}
