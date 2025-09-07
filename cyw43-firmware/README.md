# CYW43 ファームウェア配置ガイド

このディレクトリには Raspberry Pi Pico W の無線チップ（CYW43）用ファームウェアを配置します。ビルド時に `include_bytes!` で取り込むため、以下の2ファイルを配置してください。

- 43439A0.bin
- 43439A0_clm.bin
- 43439A0_btfw.bin

入手元（Embassy リポジトリ内）:
- https://github.com/embassy-rs/embassy/tree/main/cyw43-firmware

ライセンス:
- Infineon Permissive Binary License（上記リンク先の LICENSE を参照してください）

配置後のパス例:
- `cyw43-firmware/43439A0.bin`
- `cyw43-firmware/43439A0_clm.bin`
- `cyw43-firmware/43439A0_btfw.bin`

注意:
- ファイルが存在しない場合、`src/main.rs` の `include_bytes!` によりビルドエラーになります。
- 取得スクリプト例
  ```bash
  curl -LO https://raw.githubusercontent.com/embassy-rs/embassy/main/cyw43-firmware/43439A0.bin
  curl -LO https://raw.githubusercontent.com/embassy-rs/embassy/main/cyw43-firmware/43439A0_clm.bin
  curl -LO https://raw.githubusercontent.com/embassy-rs/embassy/main/cyw43-firmware/43439A0_btfw.bin
  ```
