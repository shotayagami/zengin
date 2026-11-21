# TLS で包んで使う (SSL/TLS 方式)

本実装は、通信を暗号化しない (依存クレートを持たない方針のため)。インターネットを経由させるときは、
TCP の外側を TLS で包む「TLS 終端」を前に置く。全銀の電文には手を加えないので、本実装の変更は要らない。

これは、インターネットEDI普及推進協議会 (JiEDIA) の「全銀協標準通信プロトコル (TCP/IP手順・広域IP網)
利用ガイドライン SSL/TLS 方式編」が示す形の一つ (SSL アクセラレータと全銀パッケージの組み合わせ) にあたる。
同じ形の市販品 (たとえば Biware 全銀TCPクライアントの SSL/TLS 設定) とも、そのままつながる。

Tailscale (WireGuard) の網の中だけで使うなら、Tailscale が暗号化するので、TLS は要らない。

## 構成

```
一次局 (zengin send/fetch)                                    二次局 (zengin bank serve)
   │ 平文                                                             ▲ 平文
   ▼                                                                  │
 TLS 終端 (client)  ── TLS (インターネット) ──▶  TLS 終端 (server) ──┘
```

- 二次局の前に置く TLS 終端は、サーバ証明書を持つ。ポート番号は、ガイドラインでは 5020 か、当事者間で決めたもの。
- 一次局が市販品 (TLS に対応したもの) なら、一次局側の TLS 終端は要らない。
- TLS 1.2 以上を使う。

## 方法1: stunnel

二次局側 (`stunnel-server.conf`)。TLS の 5020 で受け、同じマシンの `zengin bank serve` (平文、15021) へ渡す。

```ini
[zengin]
accept  = 0.0.0.0:5020
connect = 127.0.0.1:15021
cert    = /etc/zengin/server.crt
key     = /etc/zengin/server.key
sslVersionMin = TLSv1.2
; クライアント証明書を必須にする場合
; CAfile = /etc/zengin/client-ca.crt
; verifyChain = yes
```

一次局側 (`stunnel-client.conf`)。`zengin send --to 127.0.0.1:5021` を、相手の TLS へつなぐ。

```ini
[zengin]
client  = yes
accept  = 127.0.0.1:5021
connect = bank.example.jp:5020
CAfile  = /etc/zengin/server-ca.crt
checkHost = bank.example.jp
verifyChain = yes
sslVersionMin = TLSv1.2
```

`zengin bank serve` の `listen` は `127.0.0.1:15021` にして、平文のポートを外へ出さないこと。

## 方法2: scripts/tls-relay.py

stunnel が使えない環境用に、Python の標準ライブラリだけで書いた TLS 終端を用意している。

```sh
# 試験用の自己署名証明書
openssl req -x509 -newkey rsa:2048 -nodes -keyout server.key -out server.crt -days 30 \
    -subj "/CN=localhost" -addext "subjectAltName=DNS:localhost,IP:127.0.0.1"

# 二次局側: TLS の 25021 で受けて、平文で銀行ノード (15021) へ
python scripts/tls-relay.py server 25021 127.0.0.1:15021 --cert server.crt --key server.key

# 一次局側: 平文の 25031 で受けて、TLS で 25021 へ
python scripts/tls-relay.py client 25031 127.0.0.1:25021 --ca server.crt
zengin send --conf examples/companyA.conf --file a.dat --to 127.0.0.1:25031
```

`server` に `--ca` を付けると、クライアント証明書を必須にする。`client` の `--insecure` は、サーバ証明書を検証しない (試験用)。

確かめたこと (2026-11-21、Windows、Python 3.13 / OpenSSL 3.5): 上の組み合わせで、総合振込の送信と振込入金通知の取得が
TLS 1.3 で正常に終わる。平文のまま TLS のポートへつなぐと、TLS 終端が接続を閉じる。

## 証明書と全銀の認証

TLS は通信路の暗号化と、サーバ (必要ならクライアント) の確認をする。全銀のセンター確認コード、パスワード、
ファイルアクセスキーの検査は、これまでどおり本実装が行う。どちらかが通らなければ接続は失敗する。
