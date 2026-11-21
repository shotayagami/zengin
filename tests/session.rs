//! 一次局と二次局を、ローカルの TCP でつないで確かめる。

use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use zengin::charcode::Code;
use zengin::control::{CenterCode, FileCtl, OK, make_name};
use zengin::handler::{Handler, InFile, OutFile, PeerCfg};
use zengin::record::{FileKind, fmt};
use zengin::session::{Done, Job, Opts, fetch_file, run, send_file, serve, tune};
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
    fn find_peer(&mut self, remote: &CenterCode, own: &CenterCode) -> Result<PeerCfg, u8> {
        let c = server_cfg();
        if c.own != *own {
            Err(0x11)
        } else if c.remote != *remote {
            Err(0x12)
        } else {
            Ok(c)
        }
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
        serve(s, &mut hh.clone(), &opts)
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
    let got = fetch_file(
        connect(srv.addr, &o),
        &client_cfg(),
        &clock,
        make_name("5020", "0001", 1),
        200,
        false,
        &o,
    )
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
    let got = fetch_file(
        connect(srv.addr, &o),
        &client_cfg(),
        &clock,
        make_name("5020", "0001", 1),
        200,
        false,
        &o,
    )
    .unwrap();
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
        // 要求の当方センター確認コード (送信側=一次局自身) が登録にない → 12
        (
            PeerCfg::new("x", cc("00000000020009"), cc("00000000010001"), "PASS01", "KEY001").unwrap(),
            0x12,
        ),
        // 要求の相手センター確認コード (受信側=二次局) が違う → 11
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

#[test]
fn shokai_start_response_echoes_requested_name_and_compression() {
    use zengin::control::{CommCtl, OPEN_REQ, START_REQ, START_RSP};
    use zengin::sublayer::Link;
    use zengin::text::build_control;
    // Biware は、開始回答のファイル名 (サイクルを含む) が要求と違うと打ち切る。二次局が持っているファイルの
    // 名前や圧縮の有無ではなく、要求の値で返すこと
    let f = fmt(FileKind::Nyukin);
    let data = f
        .build(
            Code::Jis,
            f.new_header().set_num("bank_no", 2),
            &[f.new_data().set_num("amount", 500)],
        )
        .unwrap();
    let of = OutFile {
        name: make_name("5020", "0001", 2),
        rec_len: 200,
        code: Code::Jis,
        data,
        compress: false,
    };
    let srv = start(
        Test {
            outbox: Some(of),
            ..Test::default()
        },
        Opts::default(),
    );
    let cfg = client_cfg();
    let open = CommCtl {
        kind: OPEN_REQ,
        result: 0,
        peer: cfg.remote,
        own: cfg.own,
        time: [0x26, 0x11, 0x21, 0x14, 0x42, 0x14],
        password: cfg.pw_open_req,
        app: 0,
        mode: 1,
    };
    let mut link = Link::new(TcpStream::connect(srv.addr).unwrap(), 0);
    link.send(&build_control(&open.encode()), true).unwrap();
    link.recv().unwrap();
    let req = FileCtl::new(START_REQ, make_name("5020", "0001", 1), cfg.key, 200, true);
    link.send(&build_control(&req.encode()), true).unwrap();
    let rsp = FileCtl::decode(&link.recv().unwrap()[5..]).unwrap();
    assert_eq!((rsp.kind, rsp.result), (START_RSP, OK));
    assert_eq!((rsp.name, rsp.compress), (req.name, true));
    drop(link);
    let _ = srv.t.join();
}

#[test]
fn shokai_can_start_with_resend_request() {
    use zengin::control::{CLOSE_REQ, CommCtl, END_REQ, END_RSP, NO_FILE, OPEN_REQ, RETRY_REQ, START_RSP};
    use zengin::sublayer::Link;
    use zengin::text::{build_control, parse_data, parse_ttc};
    // 前回の照会が途中で終わったとき、一次局は開始要求の代わりに再送要求 (全ファイル) で始める (Biware の「再受信モード」)。
    // 二次局は開始回答を返さず、データ電文から送る
    let f = fmt(FileKind::Nyukin);
    let data = f
        .build(
            Code::Jis,
            f.new_header().set_num("bank_no", 2),
            &[f.new_data().set_num("amount", 500)],
        )
        .unwrap();
    let cfg = client_cfg();
    let comm = |kind: u8, pw: [u8; 6]| CommCtl {
        kind,
        result: 0,
        peer: cfg.remote,
        own: cfg.own,
        time: [0x26, 0x11, 0x21, 0x15, 0x00, 0x00],
        password: pw,
        app: 0,
        mode: 1,
    };
    let mut req = FileCtl::new(RETRY_REQ, make_name("5020", "0001", 1), cfg.key, 200, false);
    (req.from, req.to) = (1, 0xFFFF);
    let run = |outbox: Option<OutFile>| {
        let srv = start(Test { outbox, ..Test::default() }, Opts::default());
        let mut link = Link::new(TcpStream::connect(srv.addr).unwrap(), 0);
        link.send(&build_control(&comm(OPEN_REQ, cfg.pw_open_req).encode()), true).unwrap();
        link.recv().unwrap();
        link.send(&build_control(&req.encode()), true).unwrap();
        (srv, link)
    };

    let of = OutFile {
        name: req.name,
        rec_len: 200,
        code: Code::Jis,
        data: data.clone(),
        compress: false,
    };
    let (srv, mut link) = run(Some(of));
    let first = link.recv().unwrap();
    assert!(parse_ttc(&first).unwrap().data, "開始回答ではなくデータ電文が来る");
    assert_eq!(parse_data(&first, false).unwrap().1, data);
    let mut end = FileCtl::decode(&link.recv().unwrap()[5..]).unwrap();
    assert_eq!((end.kind, end.texts, end.records), (END_REQ, 1, 4));
    end.kind = END_RSP;
    link.send(&build_control(&end.encode()), true).unwrap();
    link.send(&build_control(&comm(CLOSE_REQ, cfg.pw_close_req).encode()), true)
        .unwrap();
    link.recv().unwrap();
    srv.t.join().unwrap().unwrap();
    assert_eq!(srv.h.lock().unwrap().sent_committed, 1);

    // 送るファイルがなければ、開始回答 (ファイルなし) を返す
    let (srv, mut link) = run(None);
    let rsp = FileCtl::decode(&link.recv().unwrap()[5..]).unwrap();
    assert_eq!((rsp.kind, rsp.result), (START_RSP, NO_FILE));
    drop(link);
    let _ = srv.t.join();
}

#[test]
fn fetch_with_resend_request() {
    // 一次局が再送要求で照会を始める (`fetch --resend`)。ファイルがあれば受け取り、なければ None
    let clock = Clock::default();
    let f = fmt(FileKind::Nyukin);
    let data = f
        .build(
            Code::Jis,
            f.new_header().set_num("bank_no", 2),
            &[f.new_data().set_num("amount", 500)],
        )
        .unwrap();
    let name = make_name("5020", "0001", 1);
    let of = OutFile {
        name,
        rec_len: 200,
        code: Code::Jis,
        data: data.clone(),
        compress: false,
    };
    let o = Opts::default();
    let srv = start(
        Test {
            outbox: Some(of),
            ..Test::default()
        },
        o.clone(),
    );
    let got = fetch_file(connect(srv.addr, &o), &client_cfg(), &clock, name, 200, true, &o).unwrap();
    srv.t.join().unwrap().unwrap();
    assert_eq!(got.unwrap().data, data);
    assert_eq!(srv.h.lock().unwrap().sent_committed, 1);

    let srv = start(Test::default(), o.clone());
    let got = fetch_file(connect(srv.addr, &o), &client_cfg(), &clock, name, 200, true, &o).unwrap();
    srv.t.join().unwrap().unwrap();
    assert!(got.is_none());
}

/// 生の電文で二次局と話すための道具 (B1〜B5 の試験用)。
mod raw {
    use super::*;
    use zengin::control::{CommCtl, OPEN_REQ};
    use zengin::sublayer::Link;
    use zengin::text::build_control;

    pub fn comm(kind: u8, mode: u8) -> CommCtl {
        let cfg = client_cfg();
        CommCtl {
            kind,
            result: 0,
            peer: cfg.remote,
            own: cfg.own,
            time: [0x26, 0x11, 0x21, 0x16, 0x00, 0x00],
            password: cfg.pw_open_req,
            app: 0,
            mode,
        }
    }

    /// 開局まで済ませた接続を返す。
    pub fn opened(srv: &Server, mode: u8) -> Link<TcpStream> {
        let mut link = Link::new(TcpStream::connect(srv.addr).unwrap(), 0);
        link.send(&build_control(&comm(OPEN_REQ, mode).encode()), true).unwrap();
        assert_eq!(CommCtl::decode(&link.recv().unwrap()[5..]).unwrap().result, 0);
        link
    }

    pub fn send_ctl(link: &mut Link<TcpStream>, body: &[u8; 64]) {
        link.send(&build_control(body), true).unwrap();
    }

    pub fn recv_comm(link: &mut Link<TcpStream>) -> CommCtl {
        CommCtl::decode(&link.recv().unwrap()[5..]).unwrap()
    }

    pub fn recv_file(link: &mut Link<TcpStream>) -> FileCtl {
        FileCtl::decode(&link.recv().unwrap()[5..]).unwrap()
    }
}

#[test]
fn mode_change_response_echoes_request() {
    // モード変更回答は、ほかの回答と同じく、センター確認コードを入れ替え、日時などは要求の値を返す
    use zengin::control::{MODE_REQ, MODE_RSP};
    let srv = start(Test::default(), Opts::default());
    let mut link = raw::opened(&srv, 0);
    let req = raw::comm(MODE_REQ, 1);
    raw::send_ctl(&mut link, &req.encode());
    let rsp = raw::recv_comm(&mut link);
    assert_eq!((rsp.kind, rsp.result), (MODE_RSP, OK));
    assert_eq!((rsp.peer, rsp.own), (req.own, req.peer));
    assert_eq!((rsp.time, rsp.app, rsp.mode), (req.time, req.app, req.mode));
    drop(link);
    let _ = srv.t.join();
}

#[test]
fn close_request_errors_name_the_wrong_field() {
    // B2: 閉局要求の誤りは、パスワードに限らず、どの項目が違うかで処理結果を分ける
    use zengin::control::{CLOSE_REQ, CLOSE_RSP};
    type Spoil = fn(&mut zengin::control::CommCtl);
    let cases: [(Spoil, u8); 3] = [
        (|c| c.peer = cc("00000000010009"), 0x11),
        (|c| c.own = cc("00000000020009"), 0x12),
        (|c| c.password = [0xF0; 6], 0x14),
    ];
    for (spoil, code) in cases {
        let srv = start(Test::default(), Opts::default());
        let mut link = raw::opened(&srv, 0);
        let mut req = raw::comm(CLOSE_REQ, 0);
        spoil(&mut req);
        raw::send_ctl(&mut link, &req.encode());
        let rsp = raw::recv_comm(&mut link);
        assert_eq!((rsp.kind, rsp.result), (CLOSE_RSP, code));
        assert_eq!(rsp.time, req.time);
        drop(link);
        let _ = srv.t.join();
    }
}

#[test]
fn file_name_aux_info_is_echoed() {
    // B4: ファイル名補助情報 (17 バイト) は、開始回答と終了回答で要求の値を返す
    use zengin::control::{END_REQ, END_RSP, START_REQ, START_RSP};
    use zengin::text::texts_for;
    let srv = start(Test::default(), Opts::default());
    let mut link = raw::opened(&srv, 0);
    let mut start_req = FileCtl::new(START_REQ, make_name("5020", "0121", 1), client_cfg().key, 120, false);
    start_req.aux = *b"\x80\x00ABCDEFGHIJKLMNO";
    raw::send_ctl(&mut link, &start_req.encode());
    let rsp = raw::recv_file(&mut link);
    assert_eq!((rsp.kind, rsp.result, rsp.aux), (START_RSP, OK, start_req.aux));
    let data = sogo_file(1, Code::Jis);
    let texts = texts_for(&data, 120, None).unwrap();
    for t in &texts {
        link.send(t, false).unwrap();
    }
    let mut end = start_req.clone();
    (end.kind, end.texts, end.records) = (END_REQ, texts.len() as u16, (data.len() / 120) as u32);
    raw::send_ctl(&mut link, &end.encode());
    let rsp = raw::recv_file(&mut link);
    assert_eq!((rsp.kind, rsp.result, rsp.aux), (END_RSP, OK, start_req.aux));
    drop(link);
    let _ = srv.t.join();
}

#[test]
fn zero_byte_file_is_accepted_and_not_committed() {
    // B5: データ電文なしの終了要求 (Biware の「0 バイトファイル送信許可」) は、正常として受け、何も確定しない
    use zengin::control::{CLOSE_REQ, CLOSE_RSP, END_REQ, END_RSP, START_REQ};
    let srv = start(Test::default(), Opts::default());
    let mut link = raw::opened(&srv, 0);
    let mut f = FileCtl::new(START_REQ, make_name("5020", "0121", 1), client_cfg().key, 120, false);
    raw::send_ctl(&mut link, &f.encode());
    assert_eq!(raw::recv_file(&mut link).result, OK);
    f.kind = END_REQ;
    raw::send_ctl(&mut link, &f.encode());
    let rsp = raw::recv_file(&mut link);
    assert_eq!((rsp.kind, rsp.result, rsp.texts, rsp.records), (END_RSP, OK, 0, 0));
    raw::send_ctl(&mut link, &raw::comm(CLOSE_REQ, 0).encode());
    assert_eq!(raw::recv_comm(&mut link).kind, CLOSE_RSP);
    srv.t.join().unwrap().unwrap();
    assert!(srv.h.lock().unwrap().received.is_empty(), "0 バイトのファイルは確定しない");
}

#[test]
fn multi_file_with_mode_change() {
    // 1回の接続で、2ファイルを送り (連絡)、モード変更して1ファイルを受け取る (照会)。確定は閉局のとき
    let clock = Clock::default();
    let f = fmt(FileKind::Nyukin);
    let notice = f
        .build(
            Code::Jis,
            f.new_header().set_num("bank_no", 2),
            &[f.new_data().set_num("amount", 500)],
        )
        .unwrap();
    let srv = start(
        Test {
            outbox: Some(OutFile {
                name: make_name("5020", "0001", 1),
                rec_len: 200,
                code: Code::Jis,
                data: notice.clone(),
                compress: false,
            }),
            ..Test::default()
        },
        Opts::default(),
    );
    let (a, b) = (sogo_file(2, Code::Jis), sogo_file(3, Code::Ebcdic));
    let mut second = out(b.clone(), Code::Ebcdic, true);
    second.name = make_name("5020", "0121", 2);
    let jobs = [
        Job::Send(out(a.clone(), Code::Jis, false)),
        Job::Send(second),
        Job::Fetch {
            name: make_name("5020", "0001", 1),
            rec_len: 200,
            resend: false,
        },
        // 同じ接続で同じ種類をもう一度求めても、2度は渡さない
        Job::Fetch {
            name: make_name("5020", "0001", 1),
            rec_len: 200,
            resend: false,
        },
    ];
    let o = Opts::default();
    let done = run(connect(srv.addr, &o), &client_cfg(), &clock, &jobs, &o).unwrap();
    srv.t.join().unwrap().unwrap();
    assert!(matches!(done[0], Done::Sent) && matches!(done[1], Done::Sent));
    assert!(matches!(&done[2], Done::Fetched(Some(f)) if f.data == notice));
    assert!(matches!(done[3], Done::Fetched(None)));
    let h = srv.h.lock().unwrap();
    assert_eq!(h.received.iter().map(|f| f.data.clone()).collect::<Vec<_>>(), [a, b]);
    assert_eq!(h.sent_committed, 1);
}

#[test]
fn mode_change_back_and_forth() {
    // 照会 → 連絡 → 照会 (仕様書59〜62頁の両方向)
    let clock = Clock::default();
    let srv = start(Test::default(), Opts::default());
    let fetch = Job::Fetch {
        name: make_name("5020", "0001", 1),
        rec_len: 200,
        resend: false,
    };
    let jobs = [fetch.clone(), Job::Send(out(sogo_file(1, Code::Jis), Code::Jis, false)), fetch];
    let o = Opts::default();
    let done = run(connect(srv.addr, &o), &client_cfg(), &clock, &jobs, &o).unwrap();
    srv.t.join().unwrap().unwrap();
    assert!(matches!(done[0], Done::Fetched(None)) && matches!(done[2], Done::Fetched(None)));
    assert_eq!(srv.h.lock().unwrap().received.len(), 1);
}

#[test]
fn files_are_not_committed_if_the_session_fails_midway() {
    // 2ファイル目で失敗したら、1ファイル目も確定しない (閉局まで正常に終わった伝送だけを確定する)
    let clock = Clock::default();
    let srv = start(Test::default(), Opts::default());
    let mut bad = out(sogo_file(1, Code::Jis), Code::Jis, false);
    bad.rec_len = 200;
    bad.data = vec![0x40; 200];
    let jobs = [Job::Send(out(sogo_file(1, Code::Jis), Code::Jis, false)), Job::Send(bad)];
    let o = Opts::default();
    expect_rejected(run(connect(srv.addr, &o), &client_cfg(), &clock, &jobs, &o).map(|_| ()), 0x15);
    assert!(srv.t.join().unwrap().is_err());
    assert!(srv.h.lock().unwrap().received.is_empty());
}

#[test]
fn primary_resumes_when_start_is_answered_with_resend_request() {
    // A3: 連絡モードで、開始要求に再送要求 (テキスト 2 から) が返ってきたら、その範囲から送る (仕様書 §VII (2)③⑤)
    use zengin::control::{CLOSE_REQ, CLOSE_RSP, CommCtl, END_REQ, END_RSP, OPEN_RSP, RETRY_REQ, START_REQ};
    use zengin::sublayer::Link;
    use zengin::text::{build_control, parse_ttc};
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = l.local_addr().unwrap();
    let cfg = server_cfg();
    let fake = thread::spawn(move || {
        let mut link = Link::new(l.accept().unwrap().0, 0);
        let ctl = |link: &mut Link<TcpStream>| link.recv().unwrap()[5..].to_vec();
        let mut c = CommCtl::decode(&ctl(&mut link)).unwrap();
        (c.kind, c.peer, c.own, c.password) = (OPEN_RSP, cfg.remote, cfg.own, cfg.pw_open_rsp);
        link.send(&build_control(&c.encode()), true).unwrap();
        let mut f = FileCtl::decode(&ctl(&mut link)).unwrap();
        assert_eq!(f.kind, START_REQ);
        (f.kind, f.from, f.to) = (RETRY_REQ, 2, 0xFFFF);
        link.send(&build_control(&f.encode()), true).unwrap();
        let mut seqs = Vec::new();
        loop {
            let t = link.recv().unwrap();
            let ttc = parse_ttc(&t).unwrap();
            if ttc.data {
                seqs.push(ttc.seq);
                continue;
            }
            let mut e = FileCtl::decode(&t[5..]).unwrap();
            assert_eq!(e.kind, END_REQ);
            e.kind = END_RSP;
            link.send(&build_control(&e.encode()), true).unwrap();
            break;
        }
        let mut c = CommCtl::decode(&ctl(&mut link)).unwrap();
        assert_eq!(c.kind, CLOSE_REQ);
        (c.kind, c.peer, c.own, c.password) = (CLOSE_RSP, cfg.remote, cfg.own, cfg.pw_close_rsp);
        link.send(&build_control(&c.encode()), true).unwrap();
        seqs
    });
    // 1 テキスト = 16 レコード (1920 バイト) なので、40 レコードは 3 テキスト
    let o = Opts::default();
    let file = out(sogo_file(38, Code::Jis), Code::Jis, false);
    send_file(connect(addr, &o), &client_cfg(), &Clock::default(), &file, &o).unwrap();
    assert_eq!(fake.join().unwrap(), [2, 3]);
}
