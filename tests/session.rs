//! 一次局と二次局を、ローカルの TCP でつないで確かめる。

use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use zengin::charcode::Code;
use zengin::control::{CenterCode, FileCtl, OK, make_name};
use zengin::handler::{Handler, InFile, OutFile, PeerCfg};
use zengin::record::{FileKind, fmt};
use zengin::session::{Opts, fetch_file, send_file, serve, tune};
use zengin::util::{Clock, ZError};

fn cc(s: &str) -> CenterCode {
    CenterCode::parse(s).unwrap()
}

/// 二次局側の自局コード `0000000001 0001`、一次局側 `0000000002 0002`。
fn server_cfg() -> PeerCfg {
    PeerCfg::new("client", cc("00000000010001"), cc("00000000020002"), "PASS01", "KEY001").unwrap()
}
fn client_cfg() -> PeerCfg {
    PeerCfg::new("server", cc("00000000020002"), cc("00000000010001"), "PASS01", "KEY001").unwrap()
}

#[derive(Default)]
struct Test {
    received: Vec<InFile>,
    outbox: Option<OutFile>,
    sent_committed: u32,
    validate_error: Option<u8>,
}

impl Handler for Test {
    fn find_peer(&mut self, remote: &CenterCode, own: &CenterCode) -> Option<PeerCfg> {
        let c = server_cfg();
        (c.remote == *remote && c.own == *own).then_some(c)
    }
    fn start_recv(&mut self, _: &PeerCfg, f: &FileCtl) -> Result<(), u8> {
        if f.rec_len == 120 { Ok(()) } else { Err(0x15) }
    }
    fn validate_recv(&mut self, _: &PeerCfg, _: &InFile) -> Result<(), u8> {
        self.validate_error.map_or(Ok(()), Err)
    }
    fn commit_recv(&mut self, _: &PeerCfg, f: InFile) {
        self.received.push(f);
    }
    fn start_send(&mut self, _: &PeerCfg, _: &FileCtl) -> Result<Option<OutFile>, u8> {
        Ok(self.outbox.clone())
    }
    fn commit_send(&mut self, _: &PeerCfg, _: &OutFile) {
        self.sent_committed += 1;
    }
}

struct Server {
    addr: SocketAddr,
    h: Arc<Mutex<Test>>,
    t: JoinHandle<Result<(), ZError>>,
}

fn start(h: Test, opts: Opts) -> Server {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = l.local_addr().unwrap();
    let h = Arc::new(Mutex::new(h));
    let hh = h.clone();
    let t = thread::spawn(move || {
        let (s, _) = l.accept().unwrap();
        tune(&s, &opts).unwrap();
        serve(s, &mut hh.clone(), &Clock::default(), &opts)
    });
    Server { addr, h, t }
}

fn connect(addr: SocketAddr, o: &Opts) -> TcpStream {
    let s = TcpStream::connect(addr).unwrap();
    tune(&s, o).unwrap();
    s
}

fn sogo_file(n: usize, code: Code) -> Vec<u8> {
    let f = fmt(FileKind::Sogo);
    let h = f
        .new_header()
        .set("client_code", "1234567890")
        .set("client_name", "ﾃｽﾄ")
        .set("date", "1121")
        .set_num("bank_no", 1);
    let data: Vec<_> = (0..n)
        .map(|i| {
            f.new_data()
                .set_num("bank_no", 2)
                .set_num("branch_no", 3)
                .set("acct_kind", "1")
                .set("acct_no", format!("{:07}", i + 1))
                .set("name", "ﾔﾏﾀﾞ ﾀﾛｳ")
                .set_num("amount", 100 * (i as u64 + 1))
        })
        .collect();
    f.build(code, h, &data).unwrap()
}

fn out(data: Vec<u8>, code: Code, compress: bool) -> OutFile {
    OutFile {
        name: make_name("5020", "0121", 1),
        rec_len: 120,
        code,
        data,
        compress,
    }
}

#[test]
fn renraku_all_option_combinations() {
    let clock = Clock::default();
    for (mn_c, mn_s) in [(0, 0), (3, 5), (15, 2), (4, 0)] {
        for code in [Code::Jis, Code::Ebcdic] {
            for compress in [false, true] {
                // 40 件 = 3 テキスト以上。圧縮ありでも複数テキストになる
                let data = sogo_file(40, code);
                let (oc, os) = (
                    Opts {
                        mn: mn_c,
                        ..Opts::default()
                    },
                    Opts {
                        mn: mn_s,
                        ..Opts::default()
                    },
                );
                let srv = start(Test::default(), os);
                send_file(
                    connect(srv.addr, &oc),
                    &client_cfg(),
                    &clock,
                    &out(data.clone(), code, compress),
                    &oc,
                )
                .unwrap();
                srv.t.join().unwrap().unwrap();
                let got = srv.h.lock().unwrap().received.clone();
                assert_eq!(got.len(), 1, "mn=({mn_c},{mn_s}) {code:?} compress={compress}");
                assert_eq!(got[0].data, data);
                fmt(FileKind::Sogo).parse(&got[0].data).unwrap();
            }
        }
    }
}

#[test]
fn shokai_returns_file_and_commits() {
    let clock = Clock::default();
    let f = fmt(FileKind::Nyukin);
    let d = f.new_data().set_num("amount", 500).set("client_name", "ﾌﾘｺﾐ");
    let data = f
        .build(Code::Ebcdic, f.new_header().set_num("bank_no", 2), &[d.clone(), d])
        .unwrap();
    let of = OutFile {
        name: make_name("5020", "0001", 1),
        rec_len: 200,
        code: Code::Ebcdic,
        data: data.clone(),
        compress: true,
    };
    let srv = start(
        Test {
            outbox: Some(of),
            ..Test::default()
        },
        Opts::default(),
    );
    let o = Opts::default();
    let got = fetch_file(connect(srv.addr, &o), &client_cfg(), &clock, make_name("5020", "0001", 1), 200, &o)
        .unwrap()
        .unwrap();
    srv.t.join().unwrap().unwrap();
    assert_eq!(got.data, data);
    assert_eq!(srv.h.lock().unwrap().sent_committed, 1);
}

#[test]
fn shokai_without_file() {
    let clock = Clock::default();
    let srv = start(Test::default(), Opts::default());
    let o = Opts::default();
    let got = fetch_file(connect(srv.addr, &o), &client_cfg(), &clock, make_name("5020", "0001", 1), 200, &o).unwrap();
    srv.t.join().unwrap().unwrap();
    assert!(got.is_none());
    assert_eq!(srv.h.lock().unwrap().sent_committed, 0);
}

fn expect_rejected(r: Result<(), ZError>, code: u8) {
    match r {
        Err(ZError::Rejected(c, _)) => assert_eq!(c, code, "処理結果コード"),
        other => panic!("拒否されるはず: {other:?}"),
    }
}

#[test]
fn bad_password_key_and_codes_are_rejected() {
    let clock = Clock::default();
    let o = Opts::default();
    let data = sogo_file(2, Code::Jis);
    let cases: Vec<(PeerCfg, u8)> = vec![
        (
            PeerCfg::new("x", cc("00000000020002"), cc("00000000010001"), "WRONG1", "KEY001").unwrap(),
            0x14,
        ),
        (
            PeerCfg::new("x", cc("00000000020002"), cc("00000000010001"), "PASS01", "BADKEY").unwrap(),
            0x12,
        ),
        (
            PeerCfg::new("x", cc("00000000020009"), cc("00000000010001"), "PASS01", "KEY001").unwrap(),
            0x11,
        ),
        (
            PeerCfg::new("x", cc("00000000020002"), cc("00000000010009"), "PASS01", "KEY001").unwrap(),
            0x11,
        ),
    ];
    for (cfg, code) in cases {
        let srv = start(Test::default(), o.clone());
        let r = send_file(connect(srv.addr, &o), &cfg, &clock, &out(data.clone(), Code::Jis, false), &o);
        expect_rejected(r, code);
        assert!(srv.t.join().unwrap().is_err());
        assert!(srv.h.lock().unwrap().received.is_empty());
    }
}

#[test]
fn wrong_record_length_is_rejected_at_start() {
    let clock = Clock::default();
    let o = Opts::default();
    let srv = start(Test::default(), o.clone());
    let mut file = out(vec![0x40; 200 * 3], Code::Jis, false);
    file.rec_len = 200;
    expect_rejected(send_file(connect(srv.addr, &o), &client_cfg(), &clock, &file, &o), 0x15);
    assert!(srv.t.join().unwrap().is_err());
}

#[test]
fn validation_error_means_no_commit() {
    let clock = Clock::default();
    let o = Opts::default();
    let srv = start(
        Test {
            validate_error: Some(0x99),
            ..Test::default()
        },
        o.clone(),
    );
    let r = send_file(
        connect(srv.addr, &o),
        &client_cfg(),
        &clock,
        &out(sogo_file(3, Code::Jis), Code::Jis, false),
        &o,
    );
    expect_rejected(r, 0x99);
    assert!(srv.t.join().unwrap().is_err());
    assert!(srv.h.lock().unwrap().received.is_empty());
}

#[test]
fn missing_text_is_retransmitted_in_same_connection() {
    let clock = Clock::default();
    for mn in [0, 3] {
        for drop in [1u16, 2, 3] {
            let o = Opts {
                mn,
                drop_text: Some(drop),
                ..Opts::default()
            };
            let srv = start(Test::default(), Opts { mn, ..Opts::default() });
            let data = sogo_file(40, Code::Jis); // 17+17+6+... = 3 テキスト
            send_file(
                connect(srv.addr, &o),
                &client_cfg(),
                &clock,
                &out(data.clone(), Code::Jis, false),
                &o,
            )
            .unwrap();
            srv.t.join().unwrap().unwrap();
            let got = srv.h.lock().unwrap().received.clone();
            assert_eq!(got.len(), 1, "mn={mn} drop={drop}");
            assert_eq!(got[0].data, data);
        }
    }
}

#[test]
fn unknown_first_message_closes() {
    // 開局要求でない電文で始めると、二次局は接続を切る
    let o = Opts::default();
    let srv = start(Test::default(), o.clone());
    let mut s = connect(srv.addr, &o);
    use std::io::Write;
    s.write_all(&[0, 8, 0x11, 0, 0, 0, 0, 0]).unwrap(); // 論理ACK を先に送る
    assert!(srv.t.join().unwrap().is_err());
    let _ = OK;
}

#[test]
fn file_is_committed_even_if_client_vanishes_after_close_request() {
    use zengin::control::{CLOSE_REQ, CommCtl, END_REQ, OPEN_REQ, START_REQ, now_bcd};
    use zengin::sublayer::Link;
    use zengin::text::{build_control, texts_for};
    // 閉局回答を返してから確定する方式だと、返信前後に落ちたときにファイルが消える。返す前に確定していること
    let srv = start(Test::default(), Opts::default());
    let cfg = client_cfg();
    let clock = Clock::default();
    let comm = |kind: u8, pw: [u8; 6]| CommCtl {
        kind,
        result: 0,
        peer: cfg.remote,
        own: cfg.own,
        time: now_bcd(&clock),
        password: pw,
        app: 0,
        mode: 0,
    };
    let data = sogo_file(3, Code::Jis);
    let mut link = Link::new(TcpStream::connect(srv.addr).unwrap(), 0);
    link.send(&build_control(&comm(OPEN_REQ, cfg.pw_open_req).encode()), true).unwrap();
    link.recv().unwrap();
    let mut f = FileCtl::new(START_REQ, make_name("5020", "0121", 1), cfg.key, 120, false);
    link.send(&build_control(&f.encode()), true).unwrap();
    link.recv().unwrap();
    let texts = texts_for(&data, 120, None).unwrap();
    for t in &texts {
        link.send(t, false).unwrap();
    }
    f.kind = END_REQ;
    f.texts = texts.len() as u16;
    f.records = (data.len() / 120) as u32;
    link.send(&build_control(&f.encode()), true).unwrap();
    link.recv().unwrap(); // 終了回答
    link.send(&build_control(&comm(CLOSE_REQ, cfg.pw_close_req).encode()), true)
        .unwrap();
    drop(link); // 閉局回答を読まずに切る
    let _ = srv.t.join().unwrap(); // 二次局は閉局回答の ACK を受け取れず、エラーで終わる
    assert_eq!(srv.h.lock().unwrap().received.len(), 1, "閉局回答を返す前に確定している");
    assert_eq!(srv.h.lock().unwrap().received[0].data, data);
}

#[test]
fn open_response_echoes_request_time_and_swaps_codes() {
    use zengin::control::{CommCtl, OPEN_REQ, OPEN_RSP};
    use zengin::sublayer::Link;
    use zengin::text::build_control;
    // 日立の一次局は、回答の通信年月日時分秒が要求と違うとエラーにする。自分の時計ではなく要求の値を返すこと
    let srv = start(Test::default(), Opts::default());
    let cfg = client_cfg();
    let req = CommCtl {
        kind: OPEN_REQ,
        result: 0,
        peer: cfg.remote,
        own: cfg.own,
        time: [0x08, 0x03, 0x10, 0x14, 0x30, 0x10],
        password: cfg.pw_open_req,
        app: 0,
        mode: 0,
    };
    let mut link = Link::new(TcpStream::connect(srv.addr).unwrap(), 0);
    link.send(&build_control(&req.encode()), true).unwrap();
    let rsp = CommCtl::decode(&link.recv().unwrap()[5..]).unwrap();
    assert_eq!((rsp.kind, rsp.result), (OPEN_RSP, 0));
    assert_eq!((rsp.peer, rsp.own), (req.own, req.peer));
    assert_eq!((rsp.time, rsp.app, rsp.mode), (req.time, req.app, req.mode));
    drop(link);
    let _ = srv.t.join();
}
