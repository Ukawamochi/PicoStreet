# Raspberry Pi Pico W 用 Rust スターター

このリポジトリは Raspberry Pi Pico W 専用の Rust スターターです。無印（非 W）Pico との互換性は考慮しません。実装は Embassy ベース（非同期）で、Pico W 内蔵 LED を CYW43 ドライバ経由で点滅させます。

## 特徴
- `defmt`/`defmt-rtt` による軽量ロギング対応。
- probe-rs によるデバッグ/書き込み対応（`cargo flash` 等）。
- 最小構成で L チカから開始可能。

## 前提条件
- ターゲット追加: `rustup target add thumbv6m-none-eabi`
- ツール（任意含む）:
  - UF2 生成: `cargo install elf2uf2-rs`
  - プローブ書き込み: `cargo install cargo-flash`
- Linux での USB 権限は `for-linux.md` を参照。

## ファームウェアの配置（重要）
Pico W の内蔵 LED を制御するには CYW43 のファームウェア（FW/CLM）が必要です。以下2ファイルをリポジトリ直下の `cyw43-firmware/` に配置してください。

- 43439A0.bin
- 43439A0_clm.bin

入手先（Embassy リポジトリ）: https://github.com/embassy-rs/embassy/tree/main/cyw43-firmware

配置例:
```
cyw43-firmware/43439A0.bin
cyw43-firmware/43439A0_clm.bin
```

## ビルドと書き込み
```bash
cargo build               # デバッグビルド
cargo build --release     # リリースビルド

# UF2 生成（デバッグビルドの例）
elf2uf2-rs target/thumbv6m-none-eabi/debug/main

# BOOTSEL で RPI-RP2 ドライブへ .uf2 をコピー

# プローブ利用の例（任意）
cargo flash --chip RP2040 --release
```

## ログ（defmt/RTT）
- `cargo-embed` あるいは probe-rs RTT ビューアで `defmt::println!` 出力を確認できます。

## 注意（Pico W の LED について）
- Pico W の内蔵 LED は Wi‑Fi モジュール（CYW43）経由で制御されるため、そのままでは点灯しません。
- 内蔵 LED を使う場合は `cyw43` ドライバ等の導入とコード変更が必要です（無印向けの互換対応は行いません）。

## 構成
- エントリポイント: `src/main.rs`（`no_std`/`no_main`）
- ターゲット/リンカ設定: `.cargo/config.toml`、`memory.x`
- 依存関係: `Cargo.toml`
