//! 他の実装のバイト列との突き合わせ。
//!
//! 出典: 日立ソリューションズ「全銀協手順通信プログラム TCP/IP対応版(一次局) 操作説明書」(CDR-08857)
//! 図8.1 トレースの内容。一次局 (ZGNTCP ver03-00) が送った開局要求と、受け取った開局回答の16進ダンプ。
//! 回答の処理結果は 0x12 (当方センター確認コードエラー)。

use zengin::control::{CenterCode, CommCtl, OPEN_REQ, OPEN_RSP};
use zengin::sublayer::{SH_LEN, Sh};
use zengin::text::{build_control, parse_ttc};

fn hex(s: &str) -> Vec<u8> {
    s.split_whitespace().map(|h| u8::from_str_radix(h, 16).unwrap()).collect()
}

/// SH(8) + TTC(5) + 制御電文(64) = 77 バイト。
fn frame(text: &str) -> Vec<u8> {
    let mut v = hex("00 4D 10 00 00 00 00 00 10 00 00 00 45");
    v.extend(hex(text));
    v.resize(77, 0);
    v
}

fn hitachi_open_req() -> Vec<u8> {
    frame("00 00 00 00 00 00 00 00 00 11 00 00 00 00 00 00 08 03 10 14 30 10 00 00 00 00 00 00 F0 F1")
}

fn hitachi_open_rsp() -> Vec<u8> {
    frame("01 12 11 00 00 00 00 00 00 00 00 00 00 00 00 00 08 03 10 14 30 10 00 00 00 00 00 00 F0 F1")
}

fn split(b: &[u8]) -> (Sh, CommCtl) {
    let sh = Sh::decode(b[..SH_LEN].try_into().unwrap()).unwrap();
    let text = &b[SH_LEN..];
    let ttc = parse_ttc(text).unwrap();
    assert!(!ttc.data);
    (sh, CommCtl::decode(&text[5..]).unwrap())
}

#[test]
fn encodes_open_request_byte_for_byte_like_hitachi() {
    // 判断 D1 (ビッグエンディアン)、D3 の版番号/識別子のニブル順、TTC と SH の長さの数え方を確かめる
    let req = CommCtl {
        kind: OPEN_REQ,
        result: 0,
        peer: CenterCode::parse("00000000000000").unwrap(),
        own: CenterCode::parse("11000000000000").unwrap(),
        time: [0x08, 0x03, 0x10, 0x14, 0x30, 0x10],
        password: [0; 6],
        app: 0,
        mode: 1, // 照会 (トレースの通信条件)
    };
    let text = build_control(&req.encode());
    let sh = Sh {
        len: (SH_LEN + text.len()) as u16,
        ver: 1,
        id: 0,
        af: 0,
        mn: 0,
    };
    let mut ours = sh.encode().to_vec();
    ours.extend(text);
    assert_eq!(ours, hitachi_open_req());
}

#[test]
fn hitachi_open_response_swaps_center_codes_and_echoes_the_rest() {
    // 判断 D4: 回答のセンター確認コードは、要求と入れ替わる
    let (sh, req) = split(&hitachi_open_req());
    assert_eq!((sh.len, sh.ver, sh.id), (77, 1, 0));
    let (_, rsp) = split(&hitachi_open_rsp());
    assert_eq!((rsp.kind, rsp.result), (OPEN_RSP, 0x12));
    assert_eq!((rsp.peer, rsp.own), (req.own, req.peer));
    assert_eq!(
        (rsp.time, rsp.password, rsp.app, rsp.mode),
        (req.time, req.password, req.app, req.mode)
    );
}

#[test]
fn hitachi_logical_ack() {
    let sh = Sh::decode(&[0x00, 0x08, 0x11, 0, 0, 0, 0, 0]).unwrap();
    assert_eq!((sh.len, sh.ver, sh.id, sh.af, sh.mn), (8, 1, 1, 0, 0));
}
