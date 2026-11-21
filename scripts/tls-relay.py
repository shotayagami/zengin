#!/usr/bin/env python3
"""TLS の終端と中継。全銀の電文には手を加えず、TCP の外側を TLS で包む/はがすだけ (docs/tls.md)。

JiEDIA の「広域IP網対応版全銀手順 (SSL/TLS 方式)」の SSL アクセラレータにあたる。stunnel が使えない環境用。
Python の標準ライブラリだけで動く。

    # 受ける側 (銀行ノードの前): TLS で受けて、平文で zengin bank serve へ渡す
    python scripts/tls-relay.py server LISTEN_PORT FORWARD_HOST:PORT --cert server.crt --key server.key [--ca client-ca.crt]

    # 送る側 (zengin send/fetch の前): 平文で受けて、TLS で相手へつなぐ
    python scripts/tls-relay.py client LISTEN_PORT REMOTE_HOST:PORT [--ca server-ca.crt] [--cert c.crt --key c.key] [--insecure]

TLS 1.2 以上だけを使う。`--ca` を付けた server は、クライアント証明書を必須にする。
"""

import argparse
import socket
import ssl
import sys
import threading


def pump(src, dst):
    try:
        while data := src.recv(65536):
            dst.sendall(data)
    except OSError:
        pass
    finally:
        for s in (src, dst):
            try:
                s.shutdown(socket.SHUT_RDWR)
            except OSError:
                pass


def relay(a, b):
    threading.Thread(target=pump, args=(a, b), daemon=True).start()
    threading.Thread(target=pump, args=(b, a), daemon=True).start()


def split(addr):
    host, _, port = addr.rpartition(":")
    return host, int(port)


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("mode", choices=["server", "client"])
    p.add_argument("listen", type=int)
    p.add_argument("forward")
    p.add_argument("--cert")
    p.add_argument("--key")
    p.add_argument("--ca", help="相手の証明書を検証する CA (server ではクライアント証明書を必須にする)")
    p.add_argument("--insecure", action="store_true", help="client: サーバ証明書を検証しない (試験用)")
    a = p.parse_args()

    if a.mode == "server":
        if not (a.cert and a.key):
            sys.exit("server には --cert と --key が要る")
        ctx = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        ctx.load_cert_chain(a.cert, a.key)
        if a.ca:
            ctx.load_verify_locations(a.ca)
            ctx.verify_mode = ssl.CERT_REQUIRED
    else:
        ctx = ssl.SSLContext(ssl.PROTOCOL_TLS_CLIENT)
        if a.insecure:
            ctx.check_hostname = False
            ctx.verify_mode = ssl.CERT_NONE
        elif a.ca:
            ctx.load_verify_locations(a.ca)
        else:
            ctx.load_default_certs()
        if a.cert:
            ctx.load_cert_chain(a.cert, a.key)
    ctx.minimum_version = ssl.TLSVersion.TLSv1_2

    host, port = split(a.forward)
    srv = socket.create_server(("0.0.0.0", a.listen))
    print(f"{a.mode}: listen {a.listen} -> {a.forward}", flush=True)
    while True:
        c, addr = srv.accept()
        try:
            if a.mode == "server":
                outer = ctx.wrap_socket(c, server_side=True)
                inner = socket.create_connection((host, port))
                print(f"接続 {addr[0]} ({outer.version()})", flush=True)
                relay(outer, inner)
            else:
                outer = ctx.wrap_socket(socket.create_connection((host, port)), server_hostname=host)
                print(f"接続 {addr[0]} -> {a.forward} ({outer.version()})", flush=True)
                relay(c, outer)
        except (OSError, ssl.SSLError) as e:
            print(f"接続 {addr[0]} を閉じた: {e}", flush=True)
            c.close()


if __name__ == "__main__":
    main()
