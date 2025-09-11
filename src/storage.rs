//! 簡易すれ違いログ保存（no_std, heapless）
use core::cell::RefCell;

use defmt::*;
use portable_atomic::{AtomicU32, Ordering};
use pico_w_id_beacon::format::fmt_bytes_colon;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::{Mutex, TryLockError};
use heapless::Vec;

/// すれ違いログ1件
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct EncounterLog {
    pub mac_addr: [u8; 6],
    pub timestamp: u64, // Unix秒（未取得時は0でも可）
    pub rssi: i8,
}

impl defmt::Format for EncounterLog {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(
            f,
            "{{ mac={:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}, ts={}, rssi={} }}",
            self.mac_addr[0], self.mac_addr[1], self.mac_addr[2],
            self.mac_addr[3], self.mac_addr[4], self.mac_addr[5],
            self.timestamp, self.rssi
        );
    }
}

/// ログ最大件数
pub const MAX_ENCOUNTERS: usize = 100;

// Mutexの中にRefCell<Vec<..>>を入れる（ロック後に可変借用するため）
static ENCOUNTER_BUFFER: Mutex<CriticalSectionRawMutex, RefCell<Vec<EncounterLog, MAX_ENCOUNTERS>>> =
    Mutex::new(RefCell::new(Vec::new()));

static TOTAL_SAVED: AtomicU32 = AtomicU32::new(0);

/// 連続重複の閾値（秒）。同一MACがこの秒数以内に再検出されたら連続重複としてスキップ。
const DEDUP_WINDOW_SECS: u64 = 30;

/// ログを保存（連続重複は抑制）。ロック取得に失敗した場合はfalseを返す。
pub fn save_encounter(mac_addr: [u8; 6], timestamp: u64, rssi: i8) -> bool {
    match ENCOUNTER_BUFFER.try_lock() {
        Err(TryLockError) => {
            // ロック競合時はスキップ（割り込み抑制のため）
            return false;
        }
        Ok(guard) => {
            let mut vec = guard.borrow_mut();

            // 連続重複抑制：最後の1件と比較
            if let Some(last) = vec.last() {
                if last.mac_addr == mac_addr {
                    if last.timestamp == 0 || timestamp == 0 {
                        // タイムスタンプが無いなら単純にスキップ
                        return true;
                    }
                    if timestamp.saturating_sub(last.timestamp) <= DEDUP_WINDOW_SECS {
                        return true;
                    }
                }
            }

            // 末尾へ追加。満杯なら最古を削除してから追加。
            if vec.len() == MAX_ENCOUNTERS {
                let _ = vec.remove(0);
            }
            let _ = vec.push(EncounterLog { mac_addr, timestamp, rssi });
            let total = TOTAL_SAVED.fetch_add(1, Ordering::Relaxed) + 1;
            let s = fmt_bytes_colon(&mac_addr);
            info!("保存: mac={} ts={} rssi={} (total={})", s.as_str(), timestamp, rssi, total);
            true
        }
    }
}

/// すべてのログをdefmtへ出力
pub fn dump_logs() {
    if let Ok(guard) = ENCOUNTER_BUFFER.try_lock() {
        let vec = guard.borrow();
        info!("保存件数={}件", vec.len());
        for (i, e) in vec.iter().enumerate() {
            info!("#{}, {}", i, e);
        }
    }
}

/// ログを全消去
pub fn clear() {
    if let Ok(guard) = ENCOUNTER_BUFFER.try_lock() {
        guard.borrow_mut().clear();
        info!("ログをクリアしました");
    }
}

/// 総保存件数を返す（起動後の累計）。
pub fn total_saved() -> u32 {
    TOTAL_SAVED.load(Ordering::Relaxed)
}

/// バッファのスナップショットを`out`へコピーして件数を返す。
pub fn snapshot(out: &mut heapless::Vec<EncounterLog, MAX_ENCOUNTERS>) -> usize {
    if let Ok(guard) = ENCOUNTER_BUFFER.try_lock() {
        let vec = guard.borrow();
        out.clear();
        for e in vec.iter() {
            let _ = out.push(*e);
        }
        vec.len()
    } else {
        0
    }
}
