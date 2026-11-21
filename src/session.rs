//! 電文制御: 一次局 (TCP要求側) と二次局 (TCP受諾側) の手順。docs/spec-protocol.md §4。

use crate::charcode::Code;
use crate::control::*;
use crate::handler::{Handler, InFile, OutFile, PeerCfg};
use crate::sublayer::Link;
use crate::text::{TTC_LEN, build_control, parse_data, parse_ttc, texts_for};
use crate::util::{Clock, Result, ZError, proto};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};

/// 再送要求を受けて再送する回数の上限。
const MAX_RETRIES: u32 = 3;

#[derive(Clone, Debug)]
pub struct Opts {
    /// 自局の連続受信回数 MN (0〜15)
    pub mn: u8,
    /// 無通信監視タイマ
    pub timeout: Duration,
    /// 試験用: 最初の送信でこの番号のテキストを送らない (再送要求の動作確認)
    pub drop_text: Option<u16>,
    /// 受信するファイルの大きさの上限 (バイト)。超えたら切断する
    pub max_file: usize,
    /// 待ち受け側の1接続の全体の時間の上限。1バイトずつ遅く送り続ける相手を切るため
    pub max_session: Duration,
}

impl Default for Opts {
    fn default() -> Self {
        Opts {
            mn: 3,
            timeout: Duration::from_secs(30),
            drop_text: None,
            max_file: 16 << 20,
            max_session: Duration::from_secs(300),
        }
    }
}

/// 接続全体の期限つきの TCP ストリーム。読み書きのたびに、残り時間と無通信監視タイマの短い方を設定する。
pub struct Timed {
    s: TcpStream,
    deadline: Instant,
    idle: Duration,
}

impl Timed {
    pub fn new(s: TcpStream, idle: Duration, total: Duration) -> std::io::Result<Timed> {
        s.set_nodelay(true)?;
        Ok(Timed {
            s,
            deadline: Instant::now() + total,
            idle,
        })
    }

    fn left(&self) -> std::io::Result<Duration> {
        match self.deadline.checked_duration_since(Instant::now()) {
            Some(d) if !d.is_zero() => Ok(d.min(self.idle)),
            _ => Err(std::io::Error::new(std::io::ErrorKind::TimedOut, "接続の時間の上限を超えた")),
        }
    }
}

impl Read for Timed {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.s.set_read_timeout(Some(self.left()?))?;
        self.s.read(buf)
    }
}

impl Write for Timed {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.s.set_write_timeout(Some(self.left()?))?;
        self.s.write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.s.flush()
    }
}

/// TCP 接続に無通信監視タイマを設定する。
pub fn tune(s: &TcpStream, o: &Opts) -> std::io::Result<()> {
    s.set_read_timeout(Some(o.timeout))?;
    s.set_write_timeout(Some(o.timeout))?;
    s.set_nodelay(true)
}

enum Msg {
    Comm(CommCtl),
    File(FileCtl),
    Data(u16, Vec<u8>),
}

fn recv_msg<S: Read + Write>(link: &mut Link<S>, compressed: bool) -> Result<Msg> {
    let text = link.recv()?;
    let ttc = parse_ttc(&text)?;
    if ttc.data {
        let (seq, body) = parse_data(&text, compressed)?;
        return Ok(Msg::Data(seq, body));
    }
    let body = &text[TTC_LEN..];
    if body.len() != 64 {
        return proto("制御電文が64バイトでない");
    }
    Ok(if is_comm_kind(body[0]) {
        Msg::Comm(CommCtl::decode(body)?)
    } else {
        Msg::File(FileCtl::decode(body)?)
    })
}

fn send_comm<S: Read + Write>(link: &mut Link<S>, c: &CommCtl) -> Result<()> {
    link.send(&build_control(&c.encode()), true)
}

fn send_fctl<S: Read + Write>(link: &mut Link<S>, f: &FileCtl) -> Result<()> {
    link.send(&build_control(&f.encode()), true)
}

fn rejected<T>(code: u8, what: &str) -> Result<T> {
    Err(ZError::Rejected(code, what.into()))
}

/// 一次局が送る通信制御電文 (開局要求/閉局要求)。
fn comm_req(cfg: &PeerCfg, clock: &Clock, kind: u8, mode: u8) -> CommCtl {
    let password = if kind == OPEN_REQ { cfg.pw_open_req } else { cfg.pw_close_req };
    CommCtl {
        kind,
        result: OK,
        peer: cfg.remote,
        own: cfg.own,
        time: now_bcd(clock),
        password,
        app: 0,
        mode,
    }
}

/// 一次局が受け取った通信制御回答を検査する。センター確認コードは要求と入れ替わり (判断 D4)、
/// 通信年月日時分秒・アプリケーションID・モードは要求と同じ (日立の製品は、不一致をエラーにする)。
fn check_comm_rsp(cfg: &PeerCfg, req: &CommCtl, rsp: &CommCtl, kind: u8) -> Result<()> {
    if rsp.kind != kind {
        return proto(format!("通信制御回答の電文区分が不正: 0x{:02X}", rsp.kind));
    }
    if rsp.result != OK {
        return rejected(rsp.result, "通信制御電文の処理結果が NG");
    }
    let pw = if kind == OPEN_RSP { cfg.pw_open_rsp } else { cfg.pw_close_rsp };
    if rsp.peer != cfg.own || rsp.own != cfg.remote || rsp.password != pw {
        return proto("通信制御回答のセンター確認コードまたはパスワードが不一致");
    }
    if (rsp.time, rsp.app, rsp.mode) != (req.time, req.app, req.mode) {
        return proto("通信制御回答の通信年月日時分秒・アプリケーションID・モードが要求と不一致");
    }
    Ok(())
}

fn open<S: Read + Write>(link: &mut Link<S>, cfg: &PeerCfg, clock: &Clock, mode: u8) -> Result<()> {
    let req = comm_req(cfg, clock, OPEN_REQ, mode);
    send_comm(link, &req)?;
    match recv_msg(link, false)? {
        Msg::Comm(rsp) => check_comm_rsp(cfg, &req, &rsp, OPEN_RSP),
        _ => proto("開局回答を待っているのに別の電文が来た"),
    }
}

fn close<S: Read + Write>(link: &mut Link<S>, cfg: &PeerCfg, clock: &Clock, mode: u8) -> Result<()> {
    let req = comm_req(cfg, clock, CLOSE_REQ, mode);
    send_comm(link, &req)?;
    match recv_msg(link, false)? {
        Msg::Comm(rsp) => check_comm_rsp(cfg, &req, &rsp, CLOSE_RSP),
        _ => proto("閉局回答を待っているのに別の電文が来た"),
    }
}

/// データ電文と終了要求を送り、終了回答を待つ。再送要求が来たら、範囲のテキストを再送して終了要求を出し直す。
fn send_data<S: Read + Write>(link: &mut Link<S>, base: &FileCtl, file: &OutFile, opts: &Opts) -> Result<()> {
    if file.rec_len == 0 || file.rec_len > u16::MAX as usize || file.data.len() / file.rec_len > 0xFF_FFFF {
        return proto("レコード長またはレコード数が範囲外");
    }
    let comp = file.compress.then_some(file.code == Code::Ebcdic);
    let texts = texts_for(&file.data, file.rec_len, comp)?;
    for (i, t) in texts.iter().enumerate() {
        if opts.drop_text == Some(i as u16 + 1) {
            continue;
        }
        link.send(t, false)?;
    }
    let mut retries = 0;
    loop {
        let mut e = base.clone();
        e.kind = END_REQ;
        e.result = OK;
        e.texts = texts.len() as u16;
        e.records = (file.data.len() / file.rec_len) as u32;
        send_fctl(link, &e)?;
        match recv_msg(link, file.compress)? {
            Msg::File(r) if r.kind == END_RSP => {
                return if r.result == OK {
                    Ok(())
                } else {
                    rejected(r.result, "終了回答が NG")
                };
            }
            Msg::File(r) if r.kind == RETRY_REQ => {
                retries += 1;
                let (from, to) = (r.from.max(1) as usize, (r.to as usize).min(texts.len()));
                if retries > MAX_RETRIES || from > to {
                    return proto("再送要求が不正、または回数超過");
                }
                for t in &texts[from - 1..to] {
                    link.send(t, false)?;
                }
            }
            _ => return proto("終了回答/再送要求を待っているのに別の電文が来た"),
        }
    }
}

/// データ電文を終了要求まで受け取る。テキスト番号の飛びがあれば再送要求を返す。
/// 件数が合い、`validate` も通れば終了回答(OK)を返してファイルを返す。
fn recv_data<S: Read + Write>(
    link: &mut Link<S>,
    expect: &FileCtl,
    validate: &mut dyn FnMut(&InFile) -> std::result::Result<(), u8>,
    max_bytes: usize,
) -> Result<InFile> {
    let rec_len = expect.rec_len as usize;
    let mut texts: BTreeMap<u16, Vec<u8>> = BTreeMap::new();
    let mut total = 0usize;
    let mut retries = 0;
    // 終了回答のテキスト数・レコード数は、申告された値ではなく、受信して数えた値を返す
    let rsp = |kind: u8, result: u8, texts: u16, records: u32| {
        let mut r = FileCtl::new(kind, expect.name, expect.key, expect.rec_len, expect.compress);
        r.result = result;
        r.texts = texts;
        r.records = records;
        r
    };
    loop {
        match recv_msg(link, expect.compress)? {
            Msg::Data(seq, body) => {
                if rec_len == 0 || body.len() % rec_len != 0 {
                    return proto("テキストのレコード部がレコード長で割り切れない");
                }
                total += body.len();
                if let Some(old) = texts.insert(seq, body) {
                    total -= old.len();
                }
                if total > max_bytes {
                    return proto(format!("受信ファイルが上限 ({max_bytes} バイト) を超えた"));
                }
            }
            Msg::File(e) if e.kind == END_REQ => {
                let (got_t, got_r) = (texts.len() as u16, (total / rec_len.max(1)) as u32);
                if texts.keys().any(|&s| s > e.texts) {
                    send_fctl(link, &rsp(END_RSP, TEXT_COUNT_ERR, got_t, got_r))?;
                    return rejected(TEXT_COUNT_ERR, "終了要求のテキスト数より大きい番号を受けた");
                }
                let missing: Vec<u16> = (1..=e.texts).filter(|s| !texts.contains_key(s)).collect();
                if let (Some(&from), Some(&to)) = (missing.first(), missing.last()) {
                    retries += 1;
                    if retries > MAX_RETRIES {
                        return proto("再送の回数超過");
                    }
                    let mut r = rsp(RETRY_REQ, OK, 0, 0);
                    (r.from, r.to) = (from, to);
                    send_fctl(link, &r)?;
                    continue;
                }
                let data: Vec<u8> = texts.values().flatten().copied().collect();
                if (data.len() / rec_len) as u32 != e.records {
                    send_fctl(link, &rsp(END_RSP, RECORD_COUNT_ERR, got_t, got_r))?;
                    return rejected(RECORD_COUNT_ERR, "終了要求のレコード数が不一致");
                }
                let file = InFile {
                    name: expect.name,
                    rec_len,
                    data,
                };
                if let Err(code) = validate(&file) {
                    send_fctl(link, &rsp(END_RSP, code, got_t, got_r))?;
                    return rejected(code, "受信ファイルの検査エラー");
                }
                send_fctl(link, &rsp(END_RSP, OK, got_t, got_r))?;
                return Ok(file);
            }
            _ => return proto("データ電文/終了要求を待っているのに別の電文が来た"),
        }
    }
}

/// 連絡モード: ファイルを送る (一次局)。閉局回答まで正常に終わったら Ok。
pub fn send_file<S: Read + Write>(s: S, cfg: &PeerCfg, clock: &Clock, file: &OutFile, opts: &Opts) -> Result<()> {
    if file.rec_len == 0 || file.rec_len > u16::MAX as usize || file.data.len() / file.rec_len > 0xFF_FFFF {
        return proto("レコード長またはレコード数が範囲外");
    }
    let mut link = Link::new(s, opts.mn);
    open(&mut link, cfg, clock, 0)?;
    let mut start = FileCtl::new(START_REQ, file.name, cfg.key, file.rec_len as u16, file.compress);
    send_fctl(&mut link, &start)?;
    match recv_msg(&mut link, false)? {
        Msg::File(r) if r.kind == START_RSP => {
            if r.result != OK {
                return rejected(r.result, "開始回答が NG");
            }
        }
        _ => return proto("開始回答を待っているのに別の電文が来た"),
    }
    start.kind = START_REQ;
    send_data(&mut link, &start, file, opts)?;
    close(&mut link, cfg, clock, 0)
}

/// 照会モード: ファイルを取り寄せる (一次局)。相手に送るファイルがなければ `Ok(None)`。
pub fn fetch_file<S: Read + Write>(
    s: S,
    cfg: &PeerCfg,
    clock: &Clock,
    name: [u8; 12],
    rec_len: usize,
    opts: &Opts,
) -> Result<Option<InFile>> {
    let mut link = Link::new(s, opts.mn);
    open(&mut link, cfg, clock, 1)?;
    send_fctl(&mut link, &FileCtl::new(START_REQ, name, cfg.key, rec_len as u16, false))?;
    let expect = match recv_msg(&mut link, false)? {
        Msg::File(r) if r.kind == START_RSP => match r.result {
            OK => r,
            NO_FILE => {
                close(&mut link, cfg, clock, 1)?;
                return Ok(None);
            }
            c => return rejected(c, "開始回答が NG"),
        },
        _ => return proto("開始回答を待っているのに別の電文が来た"),
    };
    if expect.rec_len as usize != rec_len {
        return proto(format!("開始回答のレコード長が期待と違う: {}", expect.rec_len));
    }
    let file = recv_data(&mut link, &expect, &mut |_| Ok(()), opts.max_file)?;
    close(&mut link, cfg, clock, 1)?;
    Ok(Some(file))
}

/// 二次局として1接続を処理する。
pub fn serve<S: Read + Write>(s: S, h: &mut dyn Handler, clock: &Clock, opts: &Opts) -> Result<()> {
    let mut link = Link::new(s, opts.mn);
    // S1: 開局要求
    let Msg::Comm(req) = recv_msg(&mut link, false)? else {
        return proto("開局要求を待っているのに別の電文が来た");
    };
    if req.kind != OPEN_REQ {
        return proto("最初の電文が開局要求でない");
    }
    // 要求の「相手」は自局、「当方」は要求元 (判断 D4)
    let peer = h.find_peer(&req.own, &req.peer);
    let result = match &peer {
        None => PEER_CODE_ERR,
        Some(p) if req.password != p.pw_open_req => PASSWORD_ERR,
        Some(_) if req.app != 0 => APP_ERR,
        Some(_) if req.mode > 1 => MODE_ERR,
        Some(_) => OK,
    };
    let rsp = CommCtl {
        kind: OPEN_RSP,
        result,
        peer: req.own,
        own: req.peer,
        time: req.time, // 回答は要求の日時を返す (日立のトレースで確認)
        password: peer.as_ref().map_or([0; 6], |p| p.pw_open_rsp),
        app: req.app,
        mode: req.mode,
    };
    send_comm(&mut link, &rsp)?;
    let Some(peer) = peer.filter(|_| result == OK) else {
        return rejected(result, "開局要求を拒否した");
    };
    let mode = req.mode;

    let mut started = false;
    let mut received: Option<InFile> = None;
    let mut sent: Option<OutFile> = None;
    loop {
        match recv_msg(&mut link, false)? {
            Msg::File(f) if f.kind == START_REQ && !started => {
                started = true;
                let check = if f.key != peer.key {
                    Err(FILE_KEY_ERR)
                } else if mode == 0 {
                    h.start_recv(&peer, &f)
                } else {
                    Ok(())
                };
                let mut rsp = FileCtl::new(START_RSP, f.name, f.key, f.rec_len, f.compress);
                if let Err(code) = check {
                    rsp.result = code;
                    send_fctl(&mut link, &rsp)?;
                    return rejected(code, "開始要求を拒否した");
                }
                if mode == 0 {
                    send_fctl(&mut link, &rsp)?;
                    received = Some(recv_data(&mut link, &f, &mut |inf| h.validate_recv(&peer, inf), opts.max_file)?);
                } else {
                    match h.start_send(&peer, &f) {
                        Err(code) => {
                            rsp.result = code;
                            send_fctl(&mut link, &rsp)?;
                            return rejected(code, "照会の開始要求を拒否した");
                        }
                        Ok(None) => {
                            rsp.result = NO_FILE;
                            send_fctl(&mut link, &rsp)?;
                        }
                        Ok(Some(of)) => {
                            let base = FileCtl::new(START_RSP, of.name, peer.key, of.rec_len as u16, of.compress);
                            send_fctl(&mut link, &base)?;
                            send_data(&mut link, &base, &of, opts)?;
                            sent = Some(of);
                        }
                    }
                }
            }
            Msg::Comm(c) if c.kind == CLOSE_REQ => {
                let ok = c.own == peer.remote && c.peer == peer.own && c.password == peer.pw_close_req;
                let rsp = CommCtl {
                    kind: CLOSE_RSP,
                    result: if ok { OK } else { PASSWORD_ERR },
                    peer: c.own,
                    own: c.peer,
                    time: c.time,
                    password: peer.pw_close_rsp,
                    app: c.app,
                    mode: c.mode,
                };
                // 受け取ったファイルは、閉局回答を返す前に確定する。返したあとに落ちると、送信側は成功と思って
                // ファイルを捨て、受信側にも残らず、データが消えるため。回答が届かず送信側が再送した場合の
                // 二重処理は、ハンドラが重複を無視して防ぐ (Handler の説明を参照)。
                if ok {
                    if let Some(f) = received.take() {
                        h.commit_recv(&peer, f);
                    }
                }
                send_comm(&mut link, &rsp)?;
                if !ok {
                    return rejected(PASSWORD_ERR, "閉局要求を拒否した");
                }
                // 送ったファイル (照会) は、相手が閉局回答を受け取ってから確定する。受け取れなければ、次の取得で同じものを返す
                if let Some(f) = sent.take() {
                    h.commit_send(&peer, &f);
                }
                return Ok(());
            }
            Msg::Comm(c) if c.kind == MODE_REQ => {
                let mut rsp = comm_req(&peer, clock, MODE_RSP, c.mode);
                rsp.result = MODE_CHANGE_NG;
                (rsp.peer, rsp.own) = (c.own, c.peer);
                send_comm(&mut link, &rsp)?;
                return rejected(MODE_CHANGE_NG, "モード変更は未対応");
            }
            _ => return proto("想定外の電文"),
        }
    }
}
