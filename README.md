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
**BLE動作**:
- **送信表示**: 1秒毎100ms点灯
- **受信表示**: 検出毎に高速5回点滅
- **エラー表示**: 500ms・500ms・100ms点滅の繰り返し

**WiFi動作**:
- **接続中**: 500ms間隔点滅
- **接続完了**: 2秒点灯
- **接続失敗**: 100ms高速点滅×5回
- **テスト成功**: 120ms短点滅×3回

## 🔧 ハードウェア
- **LED**: 内蔵LED（WL_GPIO0）のみ使用、外付け配線不要
- **WiFi**: CYW43チップ内蔵、接続テスト機能付き

## ⚙️ 開発環境
```bash
# ターゲット追加
rustup target add thumbv6m-none-eabi

# ビルド・実行  
cargo build --release
cargo run --release  # probe-rs経由
```

## 💾 メモリ使用量
| ビルド種別 | Flash使用量 | RAM使用量 | Flash使用率 |
|-----------|------------|-----------|------------|
| **リリース版** | 459 KB | 33.5 KB | 22.4% |
| デバッグ版 | 944 KB | 33.2 KB | 46.1% |

### リリース版が小さい理由
- **コード最適化**: デッドコード除去、インライン展開
- **デバッグ情報削除**: シンボル・行番号・型情報を除去
- **LTO**: リンク時最適化でクロスモジュール最適化

**📊 仕様**: Pico W Flash 2MB / RAM 264KB

## 📋 プロトコル（簡素化版）
```
Service UUID: 0xF00D
Payload: [Ver][Type][BD_ADDR ×6]
         0x01  0x50   MACアドレス
```

## 🏗️ コード構成
- `main.rs` - システム初期化、CYW43制御
- `ble.rs` - BLE送受信の並列処理
- `wifi.rs` - WiFi接続・ネットワークテスト
- `adv_payload.rs` - BLEペイロード生成・解析
- `device_id.rs` - MACアドレス取得
- `leds.rs` - 内蔵LED制御
- `format.rs` - MACアドレス表示フォーマット
- `lib.rs` - 共通定数・モジュール定義
- `wifi_config.rs` - WiFi認証情報（要設定）

## 🎯 動作確認
### 事前準備
1. `wifi_config.rs` にWiFi認証情報を設定

### 実行手順
1. 2台のPico Wで同時起動
2. 起動時に内蔵LEDが3回点滅（150ms間隔）
3. WiFi接続中は500ms間隔点滅→接続完了で2秒点灯
4. HTTPテスト成功で3回短点滅
5. BLE送信中は1秒毎に100ms点灯
6. 他のPico W検出時に高速5回点滅
7. RTTログで `RECV bd_addr=XX:XX:XX:XX:XX:XX` を確認

**注意**: このデバイスはMACアドレスの検出・ログ出力のみを行います。アカウント連携等の機能は含まれていません。

---
**技術**: Rust/Embassy + trouble-host BLE + CYW43ファームウェア

