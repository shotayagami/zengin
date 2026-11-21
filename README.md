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
- Encryption is not built in; deploy inside a private overlay such as Tailscale/WireGuard, or put a TLS terminator in front (`docs/tls.md`).
- The protocol specification is copyrighted by the JBA and is **not** included. `scripts/fetch-refs.sh` downloads it for personal study.
- Documentation (mostly Japanese) is in [`docs/`](docs/). Licensed under MIT OR Apache-2.0.

Quick start: see [Run it on one machine](#1台で全部動かす) below.

## 日本語

全銀協標準通信プロトコル(TCP/IP手順・広域IP網)の学習用実装と、その上に載せた模擬の**全銀ネット(中央)**・**銀行ノード**。
企業 → 仕向銀行 → 全銀ネット → 被仕向銀行 → 企業 の振込の流れを、手元のネットワークで再現できる。

> **注意**
> - 銀行業務のシステムではない。全国銀行協会(全銀協)・全銀ネットとは無関係で、実在の銀行や全銀システムには接続しない。実際の資金や口座には使わないこと。
> - 銀行ノードは、通信サーバと口座・業務処理を1つにまとめた模擬で、口座は設定ファイル、状態はテキストファイルで持つ。実際の構成との違いは [`docs/architecture.md`](docs/architecture.md)。
> - 全銀システムと銀行の間の電文仕様は公開されていないため、為替通知(銀行 ⇔ 全銀ネット)は**独自設計**で、実システムとは互換性がない。
> - アプリ側に暗号化・強い認証はない。Tailscale (WireGuard) などの閉じた網の中で使うか、TLS 終端を前に置く ([`docs/tls.md`](docs/tls.md))。
> - 仕様書の著作権は全銀協にあり、このリポジトリには含めない。`scripts/fetch-refs.sh` で、私的な学習のために取得できる。

### できること

| 層 | 内容 |
|---|---|
| 全銀協プロトコル | サブレイヤ(論理ACK、連続送信 MN/AF)、伝送テキスト(TTC)、制御電文(開局/閉局/開始/終了/再送)、JIS・EBCDIC のレコード、繰り返し文字圧縮、同一接続内の再送 |
| 業務 | 別冊の 32 業務すべてのレコード (総合振込、給与・賞与振込、口座振替、振込口座照会、入出金取引明細、残高通知、財形、借入金、外国為替など)。`zengin kinds` で一覧 |
| 銀行ノード | 取引先企業と口座台帳、他行宛の振込を為替通知にして送信、受信した為替通知の入金、返却、組戻し、振込入金通知・入出金取引明細・残高通知の生成、預金口座振替、振込口座照会、財形の積立、担当者が登録したファイルの照会での受け渡し |
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
# (send に --fetch OUT を付けると、送信と取得を1回の接続で行う。--file は重ねて指定できる)
$Z fetch --conf examples/companyB.conf --out $D/nyukin.dat
# 受け取りに失敗した、または同じものをもう一度ほしいときは --resend (再送要求で始める)

$Z net report --conf examples/net.conf --data $D/net      # 仕向超過額と配信待ち
$Z net settle --data $D/net                               # 清算
$Z bank statement --conf examples/bank1.conf --data $D/b1 # 口座と送金の状態
$Z bank recall --data $D/b1 取扱番号                       # 組戻依頼 (取扱番号は statement で確認)
```

存在しない口座宛の 100 円は、銀行2が返却し、銀行1の口座に戻る。コマンドの一覧は `zengin` を引数なしで実行すると出る。

### ほかの業務

```sh
$Z kinds                                   # 業務の一覧 (名前、データコード、モード、レコード長)
# 預金口座振替: ACME が、銀行1の TANAKA の口座から 3000 円を引き落とす依頼を出し、結果を取る
$Z gen furikae --out $D/fk.dat --client-code 1 --client-name "ｶ)ｱｸﾐ" --bank 1 --item 1:10:1:2:ﾀﾅｶ:3000
$Z send  --conf examples/companyA.conf --kind furikae --file $D/fk.dat
$Z fetch --conf examples/companyA.conf --kind furikaekekka --out $D/fk-result.dat
$Z fetch --conf examples/companyA.conf --kind meisai --out $D/meisai.dat     # 入出金取引明細
$Z fetch --conf examples/companyA.conf --kind zandaka --out $D/zandaka.dat   # 残高通知
# 項目を直接指定して作る (--set はヘッダー、--rec はデータ・レコード1件)
$Z gen zaikei --out $D/zk.dat --set client_code=1 --rec "member_code=11;member_name=ﾔﾏﾀﾞ;salary_amount=10000"
# 銀行の担当者が、照会で渡すファイルを登録する (住宅ローン、借入金、外国為替など)
$Z bank publish --data $D/b1 --company ACME --kind loanzaikei --file loan.dat
$Z show --kind meisai $D/meisai.dat        # ファイルの中身を表示する
```

業務ごとの銀行ノードの処理は [`docs/spec-net.md`](docs/spec-net.md) §4.8。

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
  `[company 名前]`・`[account 名前]` の `name` は口座の名義 (半角カナ・英大文字など。明細や通知のファイルに入り、振込口座照会で照らし合わせる)。省略時はセクション名を名義にするので、セクション名に英小文字などを使うときは `name` を指定する。

### ドキュメント

| ファイル | 内容 |
|---|---|
| [`docs/spec-protocol.md`](docs/spec-protocol.md) | プロトコルの実装仕様 (バイトレイアウト、状態遷移、判断した点) |
| [`docs/spec-net.md`](docs/spec-net.md) | 全銀ネットと銀行ノードの仕様 (為替通知の独自フォーマットを含む) |
| [`docs/protocol-notes.md`](docs/protocol-notes.md) | 仕様書の読みのメモ |
| [`docs/tcpip-wide-ip.md`](docs/tcpip-wide-ip.md) | TCP/IP手順・広域IP網の背景と資料の所在 |
| [`docs/tls.md`](docs/tls.md) | TLS で包んで使う方法 (stunnel、`scripts/tls-relay.py`) |
| [`docs/architecture.md`](docs/architecture.md) | 実際の銀行の一般的な構成 (対外接続、ファイル伝送サーバ、業務システム、勘定系) と、このプロジェクトでの簡略化 |
| [`docs/zengin-system.md`](docs/zengin-system.md) | 全銀システムの仕組みの調査 |

### 開発

```sh
cargo test                                   # 単体、結合、堅牢性、状態の復元、バイナリの結合
cargo fmt --check && cargo clippy --all-targets -- -D warnings
```

CI の設定は `.github/workflows/ci.yml` (Linux・Windows・macOS、Rust 1.85)。貢献の方法は [`CONTRIBUTING.md`](CONTRIBUTING.md)。

### 未確認・対象外

- 仕様書だけでは決まらない読み (バイト順、SH のニブル順、開局回答のセンター確認コード、表の ○/△ の意味) は、市販製品の説明書の記述や16進ダンプで確認した (`docs/spec-protocol.md` §5、`tests/interop.rs`)。
  市販の全銀TCPクライアント (インターコム Biware、評価版) と本実装の銀行ノードの間で、総合振込の送信 (連絡モード) と振込入金通知の受信 (照会モード) が、圧縮あり・なしとも正常に終わることも確かめた。
  EBCDIC のデータ、再送要求で始める受信のやり直しも同じ。試していないのは、本実装が一次局になる向きなど (`docs/spec-protocol.md` §7)。`scripts/tap.py` で通信を中継すると、全メッセージの16進ダンプを取れる。
- 回線が切れたファイルを、接続し直したときに続きから受ける機能は、`--resume` を付けたときだけ有効 (`docs/spec-protocol.md` §7)。
- 市販品の拡張 (可変長レコード、32,767 バイトまでのテキスト) にも対応する (`docs/spec-protocol.md` §3.7)。サービス時間帯 (`--hours`)、二重ファイル伝送の拒否 (銀行の設定 `duplicate = reject`) も選べる。
- 対象外: 本体での暗号化 (TLS 終端を前に置く。`docs/tls.md`)、取組日の検査、手数料、担保、稼働時間。
- 状態は、処理ごとに全体を1つのファイル (`state.snapshot`) に書き、一時ファイルからの1回の置き換えで更新する。処理の途中でプロセスが落ちても、一部だけ書けた状態は残らない (落ちる位置をすべて試す試験 `tests/crash.rs` がある)。データベースではないので、状態が大きいと書き直しが遅くなる。壊れた状態ファイルは、初期状態に戻さず、起動を失敗させる。
- 受け取ったファイルは、閉局回答を返す前に確定する (返したあとに落ちてもデータが消えないようにするため)。そのため、再送されたときの二重処理は、重複判定で防ぐ。
- 日時は日本時間 (UTC+9) 固定。和暦は令和のみ。

### セキュリティ

アプリ側の認証は、センター確認コード、6文字のパスワード、ファイルアクセスキーだけで、すべて平文。
信頼できない相手が接続できる場所に置かないこと。不具合や脆弱性は、Issue または非公開の連絡で報告してほしい([`SECURITY.md`](SECURITY.md))。

### ライセンス

[MIT](LICENSE-MIT) または [Apache-2.0](LICENSE-APACHE) のどちらかを選べる。
