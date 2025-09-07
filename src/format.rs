//! バイト列の可読フォーマット補助（defmt ログ用）

use heapless::String;

/// 6/8/16 バイト配列を "aa:bb:cc:..." 形式に整形して返す。
/// 長さに応じて素直にバイト区切りのコロン連結を行う。
pub fn fmt_bytes_colon(v: &[u8]) -> String<64> {
    let mut s: String<64> = String::new();
    for (i, b) in v.iter().enumerate() {
        if i > 0 {
            let _ = s.push(':');
        }
        let _ = push_hex_byte(&mut s, *b);
    }
    s
}

/// CONTACT_ID(16B) を短縮表示（先頭3バイト + 末尾3バイト）
/// 例: aa:bb:cc..dd:ee:ff
pub fn fmt_id16_compact(v: &[u8; 16]) -> String<64> {
    let mut s: String<64> = String::new();
    // 先頭3B
    for i in 0..3 {
        if i > 0 { let _ = s.push(':'); }
        let _ = push_hex_byte(&mut s, v[i]);
    }
    let _ = s.push_str("..");
    // 末尾3B
    for (idx, i) in (13..16).enumerate() {
        if idx > 0 { let _ = s.push(':'); }
        let _ = push_hex_byte(&mut s, v[i]);
    }
    s
}

/// 送信側/受信側の kind 表示用（0x00/0x01/0x02 → ラベル）
pub fn kind_label_u8(kind: u8) -> &'static str {
    match kind {
        0x00 => "BD6",
        0x01 => "UID8",
        0x02 => "RAND16",
        _ => "UNK",
    }
}

fn push_hex_byte(dst: &mut String<64>, b: u8) -> () {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let _ = dst.push(HEX[(b >> 4) as usize] as char);
    let _ = dst.push(HEX[(b & 0x0f) as usize] as char);
}
