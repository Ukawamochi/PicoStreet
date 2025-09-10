# PicoStreet - Pico W MACアドレス交換システム

## 🎯 概要
**Raspberry Pi Pico W** を使ったBLEすれ違い通信で **MACアドレス（BD_ADDR）の検出・交換** を行うキーホルダー型デバイス。

近くにある他のPico Wデバイスを自動検出し、そのMACアドレスをログ出力します。

## 📡 動作方式（並列処理）
```
送信: ──TX──────TX──────TX────── (1.2-1.5秒間隔)
受信: ─RX┤──RX┤──RX┤──RX┤──── (25ms周期)
      20ms 5ms 20ms 5ms...
```

### LED表示（内蔵LEDのみ）
- **送信表示**: 1秒毎100ms点灯
- **受信表示**: 検出毎に高速5回点滅
- **エラー表示**: 500ms・500ms・100ms点滅の繰り返し

## 🔧 ハードウェア
- **LED**: 内蔵LED（WL_GPIO0）のみ使用、外付け配線不要

## ⚙️ 開発環境
```bash
# ターゲット追加
rustup target add thumbv6m-none-eabi

# ビルド・実行  
cargo build --release
cargo run --release  # probe-rs経由
```

## 📋 プロトコル（簡素化版）
```
Service UUID: 0xF00D
Payload: [Ver][Type][BD_ADDR ×6]
         0x01  0x50   MACアドレス
```

## 🏗️ コード構成
- `main.rs` - システム初期化、CYW43制御
- `ble.rs` - BLE送受信の並列処理
- `adv_payload.rs` - BLEペイロード生成・解析
- `device_id.rs` - MACアドレス取得
- `leds.rs` - 内蔵LED制御
- `format.rs` - MACアドレス表示フォーマット
- `lib.rs` - 共通定数・モジュール定義

## 🎯 動作確認
1. 2台のPico Wで同時起動
2. 起動時に内蔵LEDが3回点滅（150ms間隔）
3. 送信中は1秒毎に100ms点灯
4. 他のPico W検出時に高速5回点滅
5. RTTログで `RECV bd_addr=XX:XX:XX:XX:XX:XX` を確認

**注意**: このデバイスはMACアドレスの検出・ログ出力のみを行います。アカウント連携等の機能は含まれていません。

---
**技術**: Rust/Embassy + trouble-host BLE + CYW43ファームウェア

