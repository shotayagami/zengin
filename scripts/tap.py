#!/usr/bin/env python3
"""全銀協プロトコルの通信を中継して、サブレイヤのメッセージごとに16進ダンプを出す。

他の製品と本実装の間に挟んで、実際のバイト列を確かめるための道具 (docs/spec-protocol.md §7)。

    python scripts/tap.py LISTEN_PORT FORWARD_HOST:PORT

例: 二次局が 127.0.0.1:15021 にいるとき `python scripts/tap.py 5020 127.0.0.1:15021` として、
一次局の接続先を 127.0.0.1:5020 にする。
"""

import socket
import sys
import threading


def read_exact(s, n):
    buf = b""
    while len(buf) < n:
        part = s.recv(n - len(buf))
        if not part:
            return None
        buf += part
    return buf


def pump(src, dst, tag):
    try:
        while True:
            sh = read_exact(src, 8)
            if sh is None:
                break
            size = int.from_bytes(sh[:2], "big")
            body = read_exact(src, size - 8) if size > 8 else b""
            if body is None:
                break
            # byte3 は AF と MN。どちらが上位かを確かめたいので、ニブルに分けずそのまま出す
            print(f"{tag} SH {sh.hex(' ')}  (len={size} byte2=0x{sh[2]:02X} byte3=0x{sh[3]:02X})", flush=True)
            for i in range(0, len(body), 32):
                print(f"{tag}    {body[i : i + 32].hex(' ')}", flush=True)
            dst.sendall(sh + body)
    except OSError:
        pass
    finally:
        for s in (src, dst):
            try:
                s.shutdown(socket.SHUT_RDWR)
            except OSError:
                pass


def main():
    port = int(sys.argv[1])
    host, _, fport = sys.argv[2].rpartition(":")
    srv = socket.create_server(("0.0.0.0", port))
    print(f"listen {port} -> {host}:{fport}", flush=True)
    while True:
        c, addr = srv.accept()
        print(f"--- 接続 {addr[0]}", flush=True)
        u = socket.create_connection((host, int(fport)))
        threading.Thread(target=pump, args=(c, u, "一次局→二次局"), daemon=True).start()
        threading.Thread(target=pump, args=(u, c, "二次局→一次局"), daemon=True).start()


if __name__ == "__main__":
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    main()
