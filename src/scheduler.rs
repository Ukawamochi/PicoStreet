//! 送信スケジューラ（毎日3時に送信）
use defmt::*;
use embassy_time::{Timer, Duration};
use embassy_net::Stack;

use crate::api_client::{ApiPayload, send_encounters_to_server};
use crate::storage::{EncounterLog, MAX_ENCOUNTERS, snapshot};

/// スケジューラタスクを起動（現在時刻が取得できている前提）。
#[embassy_executor::task]
pub async fn uploader_task(stack: Stack<'static>, device_id: [u8; 6]) -> ! {
    loop {
        if crate::settings::is_developer_mode() {
            // Dev: 30秒毎に送信
            Timer::after(Duration::from_secs(30)).await;
            let reported_at = crate::timekeeper::now_unix().unwrap_or(0);
            info!("[DEV] 送信タイミング到来（30秒） reported_at={}", reported_at as u32);
            let mut buf: heapless::Vec<EncounterLog, MAX_ENCOUNTERS> = heapless::Vec::new();
            let count = snapshot(&mut buf);
            if count == 0 {
                info!("[DEV] 送信対象0件（スキップ）");
                continue;
            }
            let payload = ApiPayload { device_id, encounters: &buf[..count], reported_at };
            match send_encounters_to_server(stack, &payload).await {
                Ok(()) => info!("[DEV] API送信成功 件数={}", count as u32),
                Err(e) => warn!("[DEV] API送信失敗: {}", e),
            }
        } else {
            // Prod: 毎日3時
            // 現在時刻を取得（未同期なら少し待って再試行）
            let now = match crate::timekeeper::now_unix() {
                Some(x) => x,
                None => { Timer::after(Duration::from_secs(10)).await; continue; }
            };

            // 次の3時（JST）まで待機
            let local = now + 9 * 3600;
            let sec_day = local % 86_400;
            let target = 3 * 3600u64; // 03:00:00
            let sleep = if sec_day <= target { target - sec_day } else { 86_400 - (sec_day - target) };
            info!("次の送信まで{}秒", sleep);
            Timer::after(Duration::from_secs(sleep)).await;

            let mut buf: heapless::Vec<EncounterLog, MAX_ENCOUNTERS> = heapless::Vec::new();
            let count = snapshot(&mut buf);
            if count == 0 { continue; }
            let payload = ApiPayload { device_id, encounters: &buf[..count], reported_at: now };
            match send_encounters_to_server(stack, &payload).await {
                Ok(()) => {
                    info!("送信成功。バッファをクリアします");
                    crate::storage::clear();
                }
                Err(e) => warn!("送信失敗: {}", e),
            }
        }
    }
}

/// DevモードでWiFi未接続時のハートビート（30秒毎に状況を出力）
#[embassy_executor::task]
pub async fn dev_heartbeat() -> ! {
    loop {
        if crate::settings::is_developer_mode() {
            let mut buf: heapless::Vec<EncounterLog, MAX_ENCOUNTERS> = heapless::Vec::new();
            let count = snapshot(&mut buf);
            info!("[DEV] WiFi未接続 or 初期化前。件数={}（送信スキップ）", count as u32);
            Timer::after(Duration::from_secs(30)).await;
        } else {
            Timer::after(Duration::from_secs(60)).await;
        }
    }
}

// snapshot は storage の公開APIを使用
