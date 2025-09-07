# PicoStreet BLE通信プロトコル仕様書

## プロダクト概要

PicoStreetは、**Raspberry Pi Pico Wを使ったすれ違い通信システム**です。キーホルダーのように身に着けて、近くにいる他のPico Wユーザーと**XアカウントIDを交換**できます。

### 🎯 なにができる？
- **すれ違い通信**: すれ違った人とMacアドレスを交換
- **X（Twitter）アカウント交換**: MACアドレス経由でのアカウント連携
- **キーホルダー型デバイス**: キーホルダー型で鞄とかにつけて携帯する

### 📱 システム構成
- Webアプリで自分のラズピコのMacアドレスと自分のXのアカウントを紐付ける
- Webアプリで他人のMacアドレスを入力するとその他人のXのアカウントがわかる
```
Pico W (ユーザーA) ←--BLE広告--→ Pico W (ユーザーB)
       ↓                              ↓
   MACアドレス取得                  MACアドレス取得
       ↓                              ↓
     Webアプリ ←-インターネット-→ Webアプリ
       ↓                              ↓
  Xアカウント表示                  Xアカウント表示
```

## 基本通信方式

### 📡 一方向ブロードキャスト方式
- **送信**: BLE Advertisement（広告）によるMACアドレス送信
- **受信**: BLE Scan（スキャン）による広告データの受信
- **特徴**: 超シンプル 

### ⏰ 時分割動作（将来的にはTXとRXは並列化予定）
```
TXフェーズ（5秒） → RXフェーズ（10秒） → TXフェーズ（5秒）...
```

## BLE Advertisement Data構造

### 📦 全体フレーム（最大31バイト）
```
┌─────────────┬──────────────────────────────────────┐
│   Flags     │           Service Data               │
│  (3 bytes)  │            (最大28 bytes)            │
└─────────────┴──────────────────────────────────────┘
```

### 1. Flags部分（3バイト固定）
```
[Length] [Type] [Value]
   0x02   0x01   0x06
```
- **Length**: 0x02（2バイト）
- **Type**: 0x01（Flags）
- **Value**: 0x06（LE General Discoverable + BR/EDR Not Supported）

### 2. Service Data部分（固定長）
```
[Length] [Type] [UUID_LO] [UUID_HI] [PicoStreet Payload...]
  0x0A    0x16     0x0D      0xF0    [8 bytes]
```
- **Length**: 0x0A（10バイト = UUID 2bytes + Payload 8bytes）
- **Type**: 0x16（Service Data - 16-bit UUID）
- **UUID**: 0xF00D（PicoStreet識別用）
- **Payload**: 8バイト固定（ヘッダー + MACアドレス）

## PicoStreet Payloadプロトコル
MACアドレスの交換に特化させる。ほかの機能はいらない。

### 📋 全体構造（8バイト固定）
```
┌─────────────────────────────┐
│   Header   │            BD_ADDR (MAC Address)           │
│ (2 bytes)  │                (6 bytes)                   │
└─────────────────────────────┘
```

### 1. Header部分（2バイト固定）
```
[Version] [DeviceType]
   0x01      0x50
```

| フィールド | サイズ | 値 | 説明 |
|-----------|--------|----|----- |
| Version | 1 byte | 0x01 | プロトコルバージョン |
| DeviceType | 1 byte | 0x50 | PicoStreet識別子（'P'=0x50） |

### 2. BD_ADDR部分（6バイト固定）
```
[MAC_ADDR_5] [MAC_ADDR_4] [MAC_ADDR_3] [MAC_ADDR_2] [MAC_ADDR_1] [MAC_ADDR_0]
```
- CYW43チップから取得した6バイトのBluetooth MACアドレス
- デバイス固有の識別子として使用
- Webアプリで自分のラズピコのMacアドレスと自分のXのアカウントを紐付ける
- Webアプリで他人のMacアドレスを入力するとその他人のXのアカウントがわかる

### 📋 完全なPayload例
```
0x01 0x50 0x12 0x34 0x56 0x78 0x9A 0xBC
│    │    └────────────┬────────────────┘
│    │                │
│    │                └─ BD_ADDR: 12:34:56:78:9A:BC
│    └─ DeviceType: PicoStreet (0x50)
└─ Version: 1 (0x01)
```

## PicoStreet非対応デバイス除外

### 🚫 除外対象デバイス
MACアドレスを交換するのは**PicoStreet キーホルダー同士の時のみ**です。以下は除外：

1. **スマートフォン** - iPhone/Android等の一般端末
2. **イヤホン/ヘッドホン** - AirPods/Galaxy Buds等の音響機器  
3. **スマートウォッチ** - Apple Watch/Galaxy Watch等のウェアラブル
4. **その他IoT機器** - スマート家電、車載機器等

### ✅ 識別方法
1. **Service UUID 0xF00D**をアドバタイズしているか
2. **DeviceType=0x50**を含むペイロードか
3. **8バイト固定長**のPicoStreetペイロードか

→ 上記3条件を**すべて満たす**デバイスのみ受信したIDを保存する（今はGPIO18のLEDを点滅させるだけ）

## ユーザーID統一方針

### BD_ADDR (WiFi MACアドレス)をユーザーIDとして利用

#### **ユーザー表示形式**
```
表示形式: 28:CD:C1:15:26:11 (コロン区切り6バイト)
ログ形式: id[kind=BD6] 28:CD:C1:15:26:11 rssi=-45
内部形式: [0x28, 0xCD, 0xC1, 0x15, 0x26, 0x11, 0x00, ...]
```

#### **固有性保証**
- **Raspberry Pi Trading Ltd**: OUI `28:CD:C1`で製造者確認
- **デバイス固有部**: 下位3バイトで個体識別
- **重複確率**: 実質的にゼロ（IEEE管理）

## 実装例

### 📤 送信データ例
```
Flags:        [0x02, 0x01, 0x06]
Service Data: [0x0A, 0x16, 0x0D, 0xF0,           // Service Data Header
               0x01, 0x50,                       // PicoStreet Header
               0x28, 0xCD, 0xC1, 0x15, 0x26, 0x11] // BD_ADDR (MAC)
```

**完全なAdvertisement Data（14バイト）**:
```
02 01 06 0A 16 0D F0 01 50 28 CD C1 15 26 11
│  │  │  │  │  │  │  │  │  └─────┬─────────┘
│  │  │  │  │  │  │  │  │        │
│  │  │  │  │  │  │  │  │        └─ MAC: 28:CD:C1:15:26:11
│  │  │  │  │  │  │  │  └─ DeviceType: PicoStreet (0x50)
│  │  │  │  │  │  │  └─ Version: 1
│  │  │  │  │  │  └─ Service UUID: 0xF00D (LE)
│  │  │  │  │  └─ Type: Service Data (0x16)
│  │  │  │  └─ Length: 10 bytes (UUID + Payload)
│  │  │  └─ Value: LE General Discoverable (0x06)
│  │  └─ Type: Flags (0x01)
│  └─ Length: 2 bytes
└─ AD Type: Flags
```

### 📥 受信処理
1. **Advertisement受信**: Passive ScanでRXフェーズ中に広告データ取得
2. **Service Data抽出**: UUID=0xF00DのService Dataを検索  
3. **Header検証**: Version=0x01, DeviceType=0x50を確認
4. **BD_ADDR抽出**: 6バイトのMACアドレスを取得
5. **自己除外**: 自分のBD_ADDRと比較して除外
6. **デバイス検出**: 他PicoStreetデバイスとして記録・LED点滅

## 通信パラメータ

### 📡 広告設定（TXフェーズ）
- **広告間隔**: 250ms
- **広告時間**: 5秒  
- **広告タイプ**: Non-connectable, Non-scannable Undirected

### 📱 スキャン設定（RXフェーズ）
- **スキャンタイプ**: Passive（応答要求なし）
- **スキャン間隔**: 200ms
- **スキャンウィンドウ**: 150ms  
- **スキャン時間**: 10秒

### 🔋 電力効率
- **TXデューティ比**: 33%（5秒送信/15秒周期）
- **RXデューティ比**: 67%（10秒受信/15秒周期）

### 📏 制約事項
- **BLE制限**: Advertisement Data最大31バイト
- **Payload領域**: 8バイト固定（Header 2 + BD_ADDR 6）
- **識別子**: BD_ADDR（6バイト）のみ使用

---

*PicoStreet v1.1 - 2025年9月7日* : 拡張性を捨てて実装難易度を下げた
*PicoStreet v1.0 - 2025年9月7日* : 実装の基準としてプロトコル策定 拡張性重視


