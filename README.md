# zengin

[English](#english) | [日本語](#日本語)

## English

A learning-oriented implementation of the **Zengin standard communication protocol (TCP/IP procedure, wide-area IP network edition)**
used for file exchange between companies and banks in Japan, plus a simulated **Zengin-Net hub** and **bank nodes** on top of it.

- Pure Rust, standard library only. Runs on any OS with Rust 1.85+.
- Implements the sublayer (logical ACK, burst sending), text blocks, control messages, JIS/EBCDIC records, run-length compression,
  retransmission, and the company → bank → Zengin-Net → bank → company transfer flow (netting limits, settlement, returns, recalls).
- **This is not a banking system.** It is not affiliated with the Japanese Bankers Association (JBA) or Zengin-Net, it never talks to real banks,
  and the inter-bank messages are our own design (the real ones are not public). Do not use it for real money or real bank connections.
- Encryption is not implemented; the intended deployment is inside a private overlay such as Tailscale/WireGuard.
- The protocol specification is copyrighted by the JBA and is **not** included. `scripts/fetch-refs.sh` downloads it for personal study.
- Documentation (mostly Japanese) is in [`docs/`](docs/). Licensed under MIT OR Apache-2.0.

Quick start: see [Run it on one machine](#1台で全部動かす) below.

## 日本語

全銀協標準通信プロトコル(TCP/IP手順・広域IP網)の学習用実装と、その上に載せた模擬の**全銀ネット(中央)**・**銀行ノード**。
企業 → 仕向銀行 → 全銀ネット → 被仕向銀行 → 企業 の振込の流れを、手元のネットワークで再現できる。

> **注意**
> - 銀行業務のシステムではない。全国銀行協会(全銀協)・全銀ネットとは無関係で、実在の銀行や全銀システムには接続しない。実際の資金や口座には使わないこと。
> - 全銀システムと銀行の間の電文仕様は公開されていないため、為替通知(銀行 ⇔ 全銀ネット)は**独自設計**で、実システムとは互換性がない。
> - アプリ側に暗号化・強い認証はない。Tailscale (WireGuard) などの閉じた網の中で使う前提。
> - 仕様書の著作権は全銀協にあり、このリポジトリには含めない。`scripts/fetch-refs.sh` で、私的な学習のために取得できる。

### できること

| 層 | 内容 |
|---|---|
| 全銀協プロトコル | サブレイヤ(論理ACK、連続送信 MN/AF)、伝送テキスト(TTC)、制御電文(開局/閉局/開始/終了/再送)、JIS・EBCDIC のレコード、繰り返し文字圧縮、同一接続内の再送 |
| 業務 | 総合振込、給与振込(連絡モード)、振込入金通知(照会モード) |
| 銀行ノード | 取引先企業と口座台帳、他行宛の振込を為替通知にして送信、受信した為替通知の入金、返却、組戻し、振込入金通知の生成 |
| 全銀ネット | 為替通知の中継(銀行へ自分から配信、失敗時は再試行)、仕向超過額管理、1億円以上の即時決済、日次清算、重複の無視 |

構成:

```
企業 ──(全銀協プロトコル)──▶ 仕向銀行 ──(同)──▶ 全銀ネット ──(同)──▶ 被仕向銀行 ──▶ 企業
        総合振込・給与振込        為替通知 (独自設計)   中継・仕向超過額管理・清算      振込入金通知
```

| ノード | 起動 | 役割 |
|---|---|---|
| 全銀ネット | `zengin net serve` | 銀行からの為替通知を受け、宛先の銀行へ送る |
| 銀行 | `zengin bank serve` | 企業からの振込ファイルと、全銀ネットからの為替通知を受ける |
| 企業 | `zengin send` / `zengin fetch` | 銀行へ接続して、振込ファイルを送る/振込入金通知を取る |

### 1台で全部動かす

Rust 1.85 以上が必要。依存クレートはない。

```sh
cargo build --release
Z=target/release/zengin
D=/tmp/zengin-demo

# 3つのノードを起動 (別々の端末でもよい。--interval は配信・送信の周期の秒数)
$Z net  serve --conf examples/net.conf   --data $D/net --interval 0.3 &
$Z bank serve --conf examples/bank1.conf --data $D/b1  --interval 0.3 &
$Z bank serve --conf examples/bank2.conf --data $D/b2  --interval 0.3 &

# 企業 ACME (銀行1) が、銀行2の口座 2 件と、存在しない口座 1 件へ振り込む
$Z gen sogo --out $D/sogo.dat --client-code 1 --client-name "ｶ)ｱｸﾐ" --bank 1 \
    --item 2:20:1:1:ﾍﾞﾀ:12000 --item 2:20:1:2:ｽｽﾞｷ:3500 --item 2:20:1:999:ｱﾅｼ:100
$Z send --conf examples/companyA.conf --file $D/sogo.dat --compress

# 企業 BETA (銀行2) が、振込入金通知を取りに行く
$Z fetch --conf examples/companyB.conf --out $D/nyukin.dat

$Z net report --conf examples/net.conf --data $D/net      # 仕向超過額と配信待ち
$Z net settle --data $D/net                               # 清算
$Z bank statement --conf examples/bank1.conf --data $D/b1 # 口座と送金の状態
$Z bank recall --data $D/b1 取扱番号                       # 組戻依頼 (取扱番号は statement で確認)
```

存在しない口座宛の 100 円は、銀行2が返却し、銀行1の口座に戻る。コマンドの一覧は `zengin` を引数なしで実行すると出る。

### Tailscale 網内で動かす

1. 各ノードの設定ファイルのアドレスを、Tailscale の `100.x.y.z` (または MagicDNS 名) にする。
   全銀ネットの `[bank]` の `addr`、銀行の `net_addr`、企業の `addr`。
2. 待ち受けは `listen = 0.0.0.0:5020` (既定) か、Tailscale のアドレスを指定する。
3. センター確認コード、パスワード、キーは、接続する2者の設定で一致させる。
4. 通信の暗号化と、接続できるノードの制限は、Tailscale (WireGuard、ACL) に任せる。

### 設定ファイル

`key = value` の行と、`[種類 名前]` のセクション。`#` 以降はコメント。例は [`examples/`](examples/)。

- センター確認コードは 14 桁の数字 (センターコード10桁 + CPU/端末コード4桁)。
- パスワード、キーは 6 文字以内。使える文字は、半角カナ・英大文字・数字・記号など (`src/charcodes.txt`)。
- 銀行の `[company 名前]` が取引先企業とその口座、`[account 名前]` が個人などの口座。
  口座の残高は初回だけ設定の `balance` を使い、以降は `--data` の状態ファイルが優先される。

### ドキュメント

| ファイル | 内容 |
|---|---|
| [`docs/spec-protocol.md`](docs/spec-protocol.md) | プロトコルの実装仕様 (バイトレイアウト、状態遷移、判断した点) |
| [`docs/spec-net.md`](docs/spec-net.md) | 全銀ネットと銀行ノードの仕様 (為替通知の独自フォーマットを含む) |
| [`docs/protocol-notes.md`](docs/protocol-notes.md) | 仕様書の読みのメモ |
| [`docs/tcpip-wide-ip.md`](docs/tcpip-wide-ip.md) | TCP/IP手順・広域IP網の背景と資料の所在 |
| [`docs/zengin-system.md`](docs/zengin-system.md) | 全銀システムの仕組みの調査 |

### 開発

```sh
cargo test                                   # 単体、結合、堅牢性、状態の復元、バイナリの結合
cargo fmt --check && cargo clippy --all-targets -- -D warnings
```

CI の設定は `.github/workflows/ci.yml` (Linux・Windows・macOS、Rust 1.85)。貢献の方法は [`CONTRIBUTING.md`](CONTRIBUTING.md)。

### 未確認・対象外

- バイト順、開局回答のセンター確認コードなどは、市販製品の操作説明書に載っている16進ダンプで確認した (`tests/interop.rs`)。
  確かめられていないのは、SH の AF/MN のニブル順と、仕様書の表の ○/△ の意味 (`docs/spec-protocol.md` §5 の D3、D10)。実機との相互接続は試していない。
- 対象外: モード変更、マルチファイル伝送、接続をまたぐ回復、暗号化接続、拡張Z手順、取組日の検査、手数料、担保、稼働時間。
- 状態ファイルは、処理ごとに全体を、一時ファイル経由で書き直す簡易な方式 (1ファイルずつはディスクへ書き出してから置き換えるが、複数のファイルをまとめた原子性はない)。処理の途中でプロセスが落ちると、残高と送信待ちの整合は保証しない。壊れた状態ファイルは、初期状態に戻さず、起動を失敗させる。
- 受け取ったファイルは、閉局回答を返す前に確定する (返したあとに落ちてもデータが消えないようにするため)。そのため、再送されたときの二重処理は、重複判定で防ぐ。
- 日時は日本時間 (UTC+9) 固定。和暦は令和のみ。

### セキュリティ

アプリ側の認証は、センター確認コード、6文字のパスワード、ファイルアクセスキーだけで、すべて平文。
信頼できない相手が接続できる場所に置かないこと。不具合や脆弱性は、Issue または非公開の連絡で報告してほしい([`SECURITY.md`](SECURITY.md))。

### ライセンス

[MIT](LICENSE-MIT) または [Apache-2.0](LICENSE-APACHE) のどちらかを選べる。
