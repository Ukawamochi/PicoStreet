//! デバイス固有 ID の生成・正規化
//! PicoStreet X交換特化: BD_ADDR(6B)のみ使用

/// BD_ADDRを直接取得（PicoStreet X交換用）
pub async fn get_bd_addr(ctrl: &mut cyw43::Control<'_>) -> [u8; 6] {
    ctrl.address().await
}

/// 送信用 ID を 16 バイトに正規化して返します。
/// 注意: cyw43 の MAC 取得は非同期 API のため、こちらは未使用です。
/// 実運用は `get_device_id_with_kind` (async) を使用してください。
pub fn get_normalized_contact_id(_ctrl: &cyw43::Control<'_>) -> [u8; 16] {
    // フォールバック: 起動ごとランダム（簡易、将来フラッシュ永続化に差し替え）
    random16()
}

// 以下は後方互換性のために残存（将来削除予定）
#[derive(Copy, Clone)]
pub enum DeviceIdKind { BdAddr6, Rp2040Uid8, PersistentRand16 }

impl DeviceIdKind {
    pub fn to_kind_byte(self) -> u8 {
        match self {
            DeviceIdKind::BdAddr6 => 0x00,
            DeviceIdKind::Rp2040Uid8 => 0x01,
            DeviceIdKind::PersistentRand16 => 0x02,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            DeviceIdKind::BdAddr6 => "BD6",
            DeviceIdKind::Rp2040Uid8 => "UID8",
            DeviceIdKind::PersistentRand16 => "RAND16",
        }
    }
}

/// 非同期版: BD_ADDR(6B) -> UID(8B) -> RAND(16B)
pub async fn get_device_id_with_kind(
    ctrl: &mut cyw43::Control<'_>,
) -> ([u8; 16], DeviceIdKind) {
    // 1) BD_ADDR (Wi-Fi MAC) を取得（CYW43）
    let mac6 = ctrl.address().await; // [u8;6]
    if mac6 != [0; 6] {
        let id16 = normalize6_to_16(&mac6);
        return (id16, DeviceIdKind::BdAddr6);
    }

    // 2) RP2040 Flash Unique ID (8B)
    if let Some(uid8) = rp2040_uid8() {
        let id16 = normalize8_to_16(&uid8);
        return (id16, DeviceIdKind::Rp2040Uid8);
    }

    // 3) フォールバック: ランダム 16B（将来: フラッシュ永続化）
    (random16(), DeviceIdKind::PersistentRand16)
}

fn normalize6_to_16(mac6: &[u8; 6]) -> [u8; 16] {
    let mut out = [0u8; 16];
    out[..6].copy_from_slice(mac6);
    out
}

fn normalize8_to_16(id8: &[u8; 8]) -> [u8; 16] {
    let mut out = [0u8; 16];
    out[..8].copy_from_slice(id8);
    out
}

fn random16() -> [u8; 16] {
    // 簡易: SIO タイマやカウンタ相当がないため、システムタイマとアドレスミックス
    // RP2040 のカウンタレジスタに依存しない擬似値で妥協（PoC）。
    // 将来: 真の TRNG or HMAC(Epoch) 等へ差し替え推奨。
    let t = embassy_time::Instant::now().as_ticks() as u64;
    let mut x = t ^ ((t << 13) | (t >> 7));
    let mut out = [0u8; 16];
    for chunk in out.chunks_mut(8) {
        x ^= x.rotate_left(17) ^ 0x9E3779B97F4A7C15u64;
        chunk.copy_from_slice(&x.to_le_bytes());
    }
    out
}

fn rp2040_uid8() -> Option<[u8; 8]> {
    // embassy-rp の ROM 呼び出しで SPI Flash Unique ID を取得（8B）
    // 安全な呼び出しラッパが public にないため、ここでは未使用。
    // 将来: embassy_rp::flash::Flash を受け取り、blocking_unique_id で取得する設計に変更可。
    None
}
