Pico W ID ビーコン PoC

Raspberry Pi Pico W 上で BLE アドバタイズ Service Data に TLV 形式の ID を載せて送信し、周囲の同形式アドバタイズを受信検知して LED を点滅させる PoC です。

- TX: Pico W が拡張可能な TLV フレームを Service Data(AD type 0x16) に載せて常時発信。TXフェーズ開始時に内蔵 LED (WL_GPIO0) を点灯。
- RX: 周囲の Pico W が送る同形式のアドバタイズをスキャンし、検出毎に GPIO18 の LED を 120ms 点灯。
- プロトコル: Service Data 内に ver(1), type(1), flags(1), rsv(1), TLVs...。必須 TLV は T=0x01 CONTACT_ID(16B)。

構成
- Rust/Embassy（no_std/no_main）
- embassy-executor, embassy-rp, embassy-time
- cyw43, cyw43-pio, cyw43-firmware（FW/CLM/BTFW）
- BLE Host: trouble-host
- ログ: defmt-rtt, パニック: panic-probe

配線
- RX用 LED: GPIO18 -> 抵抗(330Ω程度) -> LED -> GND
- TX用 LED: 内蔵 LED（CYW43 側 WL_GPIO0）を使用（配線不要）

ビルド前準備
- Rust ターゲット: `rustup target add thumbv6m-none-eabi`
- ツール
  - UF2 変換: `cargo install elf2uf2-rs`
  - あるいは書込: `cargo install cargo-flash`
- Linux の udev 権限設定は probe-rs の手順を参照

CYW43 ファームウェア配置
本リポジトリの `cyw43-firmware/` 配下に以下 3 ファイルを配置してください（既に同梱済み）。
- 43439A0.bin
- 43439A0_clm.bin
- 43439A0_btfw.bin
注: CI 等でファームなしでのビルド確認をしたい場合は Cargo の `skip-cyw43-firmware` フィーチャを有効化してください。

ビルド・書き込み
- デバッグビルド: `cargo build`
- リリースビルド: `cargo build --release`
- UF2 変換（デバッグ）: `elf2uf2-rs target/thumbv6m-none-eabi/debug/pico-w-id-beacon`
- 書き込み（UF2）: BOOTSEL で RPI-RP2 をマウントし、.uf2 をコピー
- probe-rs 経由で実行: `cargo run --release`（.cargo/config.toml の runner を使用）

動作
- TXフェーズ（5秒） → RXフェーズ（10秒）を繰り返します。
- TXフェーズ時: WL_GPIO0 が点灯
- RX検知時: GPIO18 の LED が 120ms 点滅
- defmt::info! で受信した CONTACT_ID を 16バイト HEX で表示

コード配置
- src/main.rs … RP2040/CYW43 初期化、Runner タスク spawn、LED/Host 準備、タイムスライス制御
- src/ble.rs … Trouble Host の生成、広告/スキャンユーティリティ、イベントハンドラ
- src/leds.rs … WL_GPIO0 と GPIO18 の点滅ヘルパ
- src/adv_payload.rs … TLV 定義・ビルダ/パーサ（ユニットテスト付き）
- src/lib.rs … 共通定数

プロトコル仕様（抜粋）
- Service UUID: 0xF00D
- フレーム: ver(=0x01), type(=0x01), flags(0x00), rsv(0x00), TLVs...
- 必須 TLV: T=0x01 CONTACT_ID (16B)
- 初期 CONTACT_ID: DEMO-DEMO-DEMO-1（16B）

テスト
純粋ロジックはホスト側で `cargo test` 可能です（adv_payload.rs）。

コーディング規約
- cargo fmt --all と cargo clippy -D warnings を推奨
- コメント・ドキュメントは日本語

既知の注意点
- TX/RXは時分割多重（同時動作はコントローラ実装依存のため未使用）
- イベントハンドラは非同期不可のため、RXを原子的カウンタに積み、タスク側で点滅処理
- CONTACT_ID の 16バイト制約のため、例示 ID を 16B に調整しています

