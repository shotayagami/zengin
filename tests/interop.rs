//! 他の実装のバイト列との突き合わせ。
//!
//! 前半の出典: 日立ソリューションズ「全銀協手順通信プログラム TCP/IP対応版(一次局) 操作説明書」(CDR-08857)
//! 図8.1 トレースの内容。一次局 (ZGNTCP ver03-00) が送った開局要求と、受け取った開局回答の16進ダンプ。
//! 回答の処理結果は 0x12 (当方センター確認コードエラー)。

use zengin::control::{CenterCode, CommCtl, END_REQ, FileCtl, OPEN_REQ, OPEN_RSP, START_REQ, make_name, six};
use zengin::sublayer::{SH_LEN, Sh};
use zengin::text::{build_control, build_data, parse_data, parse_ttc};

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

// ここから: インターコム「Biware 全銀TCPクライアント」Ver 3.7.4 (一次局、連続受信回数 3) が、本実装の二次局へ
// 総合振込を送ったときの記録 (`scripts/tap.py` で採取、2026-11-21)。通信は閉局まで正常に終わった。

#[test]
fn encodes_open_request_byte_for_byte_like_biware() {
    let req = CommCtl {
        kind: OPEN_REQ,
        result: 0,
        peer: CenterCode::parse("00000000010001").unwrap(),
        own: CenterCode::parse("00000001010001").unwrap(),
        time: [0x26, 0x11, 0x21, 0x14, 0x33, 0x28],
        password: six("CPW001").unwrap(),
        app: 0,
        mode: 0,
    };
    let text = build_control(&req.encode());
    let sh = Sh {
        len: (SH_LEN + text.len()) as u16,
        ver: 1,
        id: 0,
        af: 0,
        mn: 3,
    };
    let mut ours = sh.encode().to_vec();
    ours.extend(text);
    let mut biware = hex(
        "00 4d 10 03 00 00 00 00 10 00 00 00 45 00 00 00 00 00 00 01 00 01 00 00 00 01 01 00 01          26 11 21 14 33 28 c3 d7 e6 f0 f0 f1 f0 f0",
    );
    biware.resize(77, 0);
    assert_eq!(ours, biware);
}

#[test]
fn biware_puts_mn_in_low_nibble_and_af_in_high_nibble() {
    // 判断 D3: 開局要求は 0x03 (MN=3)、論理ACK を求めないデータ電文は 0x13 (AF=1、MN=3)
    let open = Sh::decode(&[0x00, 0x4d, 0x10, 0x03, 0, 0, 0, 0]).unwrap();
    assert_eq!((open.af, open.mn), (0, 3));
    let data = Sh::decode(&[0x00, 0xfd, 0x10, 0x13, 0, 0, 0, 0]).unwrap();
    assert_eq!((data.len, data.id, data.af, data.mn), (253, 0, 1, 3));
}

#[test]
fn decodes_biware_file_control_messages() {
    let ctl = |s: &str| {
        let mut b = hex(s);
        b.resize(64, 0);
        FileCtl::decode(&b).unwrap()
    };
    let name = make_name("5020", "0121", 1);
    let key = six("CKEY01").unwrap();
    // 開始要求。Biware は再送指定区分に 0001〜FFFF (全ファイル) を入れてくる
    let start = ctl("10 00 f5 f0 f2 f0 f0 f1 f2 f1 f0 f1 f0 f0 c3 d2 c5 e8 f0 f1 00 00 00 00 00 f0 00 78 00 01 ff ff f0");
    assert_eq!((start.kind, start.name, start.key), (START_REQ, name, key));
    assert_eq!((start.rec_len, start.compress, start.from, start.to), (120, false, 1, 0xFFFF));
    // 終了要求: テキスト 3 件、レコード 5 件
    let end = ctl("12 00 f5 f0 f2 f0 f0 f1 f2 f1 f0 f1 f0 f0 c3 d2 c5 e8 f0 f1 00 03 00 00 05 f0 00 78 00 01 ff ff f0");
    assert_eq!((end.kind, end.texts, end.records), (END_REQ, 3, 5));
}

#[test]
fn biware_compressed_text_round_trips() {
    // 圧縮ありの総合振込の最初のテキスト (レコード 2 件 = 240 バイト、JIS)。圧縮前テキスト長は TTC を含む 245
    let text = hex(
        "11 00 01 00 51 00 f5 03 31 32 31 ca 30 06 31 b6 29 b1 b8 d0 e3 20 c2 31 02 32 31 c3 30 01 31 cf          20 c3 30 cf 20 c8 30 d1 20 01 32 c3 30 01 32 cf 20 03 30 32 30 cf 20 c4 30 01 31 c6 30 04 31 cd          de c0 db 20 c7 30 01 38 c3 30 d4 20 01 30 c8 20 00",
    );
    let (seq, records) = parse_data(&text, true).unwrap();
    assert_eq!((seq, records.len()), (1, 240));
    assert_eq!(&records[..14], b"12100000000001");
    // 本実装が同じレコードを圧縮したテキストも、圧縮前テキスト長は 245 で、復元すると元に戻る
    let ours = build_data(1, &records, Some(false));
    assert_eq!(&ours[5..7], [0x00, 0xf5]);
    assert_eq!(parse_data(&ours, true).unwrap().1, records);
}

#[test]
fn biware_ebcdic_compressed_text_uses_zero_and_space_types() {
    // EBCDIC の総合振込の最初のテキスト。0x4n はゼロ (0xF0)、0x8n はスペース (0x40) の繰り返し
    let text = hex(
        "11 00 01 00 40 00 f5 02 f1 f2 c2 f1 49 06 f1 86 5d 81 88 a5 a3 c2 f1 02 f2 f1 43 01 f1 8f 43 8f          48 91 01 f2 43 01 f2 8f 03 f0 f2 f0 8f 44 01 f1 46 04 f1 a2 be 91 9b 46 c2 f1 43 94 01 f0 88 00",
    );
    let (_, records) = parse_data(&text, true).unwrap();
    assert_eq!(records.len(), 240);
    assert_eq!(&records[..19], hex("f1 f2 f1 f1 f0 f0 f0 f0 f0 f0 f0 f0 f0 f1 86 5d 81 88 a5"));
    // 本実装の圧縮も同じ種別を使う (Biware は 2 文字の繰り返しも圧縮するので、バイト列は同じにならない)
    let ours = build_data(1, &records, Some(true));
    assert!(ours.contains(&0x49) && ours.contains(&0x8f));
    assert_eq!(parse_data(&ours, true).unwrap().1, records);
}
