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
    // モード変更には専用のパスワードがないので、開局と同じものを使う
    let password = if kind == CLOSE_REQ { cfg.pw_close_req } else { cfg.pw_open_req };
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
    let pw = if kind == CLOSE_RSP { cfg.pw_close_rsp } else { cfg.pw_open_rsp };
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

/// テキスト番号の範囲 (再送要求の From/To) を、テキストの添字の範囲にする。
fn retry_range(r: &FileCtl, n: usize) -> Result<std::ops::Range<usize>> {
    let (from, to) = (r.from.max(1) as usize, (r.to as usize).min(n));
    if from > to {
        return proto("再送要求の範囲が不正");
    }
    Ok(from - 1..to)
}

/// データ電文と終了要求を送り、終了回答を待つ。再送要求が来たら、範囲のテキストを再送して終了要求を出し直す。
/// `first` は最初に送る範囲 (開始要求に再送要求が返ってきたとき)。None なら全部。
fn send_data<S: Read + Write>(link: &mut Link<S>, base: &FileCtl, file: &OutFile, first: Option<&FileCtl>, opts: &Opts) -> Result<()> {
    if file.rec_len == 0 || file.rec_len > u16::MAX as usize || file.data.len() / file.rec_len > 0xFF_FFFF {
        return proto("レコード長またはレコード数が範囲外");
    }
    let comp = file.compress.then_some(file.code == Code::Ebcdic);
    let texts = texts_for(&file.data, file.rec_len, comp)?;
    let range = match first {
        Some(r) => retry_range(r, texts.len())?,
        None => 0..texts.len(),
    };
    for (i, t) in texts.iter().enumerate().take(range.end).skip(range.start) {
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
                if retries > MAX_RETRIES {
                    return proto("再送の回数超過");
                }
                for t in &texts[retry_range(&r, texts.len())?] {
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
        r.aux = expect.aux;
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
                if (1..=e.texts).any(|s| !texts.contains_key(&s)) {
                    retries += 1;
                    if retries > MAX_RETRIES {
                        return proto("再送の回数超過");
                    }
                    // 抜けがあれば、全ファイルの再送を求める (From=1、To=全ビットオン)。仕様書の本文の再送はファイル単位
                    let mut r = rsp(RETRY_REQ, OK, 0, 0);
                    (r.from, r.to) = (1, 0xFFFF);
                    send_fctl(link, &r)?;
                    texts.clear();
                    total = 0;
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
                // 0 バイトのファイル (データ電文なしの終了要求) は、検査せずに受け付ける。中身がないので確定もしない
                if let Err(code) = if file.data.is_empty() { Ok(()) } else { validate(&file) } {
                    send_fctl(link, &rsp(END_RSP, code, got_t, got_r))?;
                    return rejected(code, "受信ファイルの検査エラー");
                }
                send_fctl(link, &rsp(END_RSP, OK, got_t, got_r))?;
                return Ok(file);
            }
            // 再送要求で始めた照会に、送るファイルがないなどの理由で開始回答 (NG) が返ってきた場合
            Msg::File(r) if r.kind == START_RSP && r.result != OK => return rejected(r.result, "開始回答が NG"),
            _ => return proto("データ電文/終了要求を待っているのに別の電文が来た"),
        }
    }
}

/// 一次局が1回の接続で行う伝送の1件。
#[derive(Clone, Debug)]
pub enum Job {
    /// 連絡モードでファイルを送る。
    Send(OutFile),
    /// 照会モードでファイルを取り寄せる。`resend` なら開始要求の代わりに再送要求 (全ファイル) で始める。
    Fetch { name: [u8; 12], rec_len: usize, resend: bool },
}

impl Job {
    fn mode(&self) -> u8 {
        match self {
            Job::Send(_) => 0,
            Job::Fetch { .. } => 1,
        }
    }
}

/// `Job` の結果。
#[derive(Debug)]
pub enum Done {
    Sent,
    /// 相手に送るファイルがなかったら None。
    Fetched(Option<InFile>),
}

/// 一次局: 1回の接続で、ジョブを順に行う (マルチファイル伝送)。連絡と照会が混ざるときは、間でモード変更をする。
/// 閉局回答まで正常に終わったら、ジョブごとの結果を返す。途中で失敗したら Err で、どの結果も確定していない。
pub fn run<S: Read + Write>(s: S, cfg: &PeerCfg, clock: &Clock, jobs: &[Job], opts: &Opts) -> Result<Vec<Done>> {
    let Some(first) = jobs.first() else {
        return proto("ジョブがない");
    };
    let mut link = Link::new(s, opts.mn);
    let mut mode = first.mode();
    open(&mut link, cfg, clock, mode)?;
    let mut done = Vec::new();
    for job in jobs {
        if job.mode() != mode {
            mode = job.mode();
            let req = comm_req(cfg, clock, MODE_REQ, mode);
            send_comm(&mut link, &req)?;
            match recv_msg(&mut link, false)? {
                Msg::Comm(rsp) => check_comm_rsp(cfg, &req, &rsp, MODE_RSP)?,
                _ => return proto("モード変更回答を待っているのに別の電文が来た"),
            }
        }
        done.push(match job {
            Job::Send(file) => {
                send_one(&mut link, cfg, file, opts)?;
                Done::Sent
            }
            Job::Fetch { name, rec_len, resend } => Done::Fetched(fetch_one(&mut link, cfg, *name, *rec_len, *resend, opts)?),
        });
    }
    close(&mut link, cfg, clock, mode)?;
    Ok(done)
}

/// 連絡モードで1ファイル送る。開始要求に再送要求が返ってきたら (相手が前回の受信の続きを求めている)、その範囲から送る。
fn send_one<S: Read + Write>(link: &mut Link<S>, cfg: &PeerCfg, file: &OutFile, opts: &Opts) -> Result<()> {
    if file.rec_len == 0 || file.rec_len > u16::MAX as usize || file.data.len() / file.rec_len > 0xFF_FFFF {
        return proto("レコード長またはレコード数が範囲外");
    }
    let start = FileCtl::new(START_REQ, file.name, cfg.key, file.rec_len as u16, file.compress);
    send_fctl(link, &start)?;
    match recv_msg(link, false)? {
        Msg::File(r) if r.kind == START_RSP && r.result != OK => rejected(r.result, "開始回答が NG"),
        Msg::File(r) if r.kind == START_RSP => send_data(link, &start, file, None, opts),
        // 仕様書 §VII (2)③⑤: 開始要求に再送要求が返ったら、再送指定された範囲を送り、終了要求を出す
        Msg::File(r) if r.kind == RETRY_REQ => send_data(link, &start, file, Some(&r), opts),
        _ => proto("開始回答を待っているのに別の電文が来た"),
    }
}

/// 照会モードで1ファイル受け取る。相手に送るファイルがなければ None。
fn fetch_one<S: Read + Write>(
    link: &mut Link<S>,
    cfg: &PeerCfg,
    name: [u8; 12],
    rec_len: usize,
    resend: bool,
    opts: &Opts,
) -> Result<Option<InFile>> {
    let mut req = FileCtl::new(START_REQ, name, cfg.key, rec_len as u16, false);
    let got = if resend {
        // 再送要求 (全ファイル) には開始回答が返らず、データ電文から来る。送るファイルがないときだけ開始回答が来る
        (req.kind, req.from, req.to) = (RETRY_REQ, 1, 0xFFFF);
        send_fctl(link, &req)?;
        recv_data(link, &req, &mut |_| Ok(()), opts.max_file)
    } else {
        send_fctl(link, &req)?;
        match recv_msg(link, false)? {
            Msg::File(r) if r.kind == START_RSP && r.result != OK => rejected(r.result, "開始回答が NG"),
            Msg::File(r) if r.kind == START_RSP && r.rec_len as usize != rec_len => {
                proto(format!("開始回答のレコード長が期待と違う: {}", r.rec_len))
            }
            Msg::File(r) if r.kind == START_RSP => recv_data(link, &r, &mut |_| Ok(()), opts.max_file),
            _ => proto("開始回答を待っているのに別の電文が来た"),
        }
    };
    match got {
        Err(ZError::Rejected(NO_FILE, _)) => Ok(None),
        r => Ok(Some(r?).filter(|f| !f.data.is_empty())),
    }
}

/// 連絡モード: ファイルを1つ送る (一次局)。閉局回答まで正常に終わったら Ok。
pub fn send_file<S: Read + Write>(s: S, cfg: &PeerCfg, clock: &Clock, file: &OutFile, opts: &Opts) -> Result<()> {
    run(s, cfg, clock, &[Job::Send(file.clone())], opts).map(|_| ())
}

/// 照会モード: ファイルを1つ取り寄せる (一次局)。相手に送るファイルがなければ `Ok(None)`。
pub fn fetch_file<S: Read + Write>(
    s: S,
    cfg: &PeerCfg,
    clock: &Clock,
    name: [u8; 12],
    rec_len: usize,
    resend: bool,
    opts: &Opts,
) -> Result<Option<InFile>> {
    let job = Job::Fetch { name, rec_len, resend };
    match run(s, cfg, clock, &[job], opts)?.pop() {
        Some(Done::Fetched(f)) => Ok(f),
        _ => proto("照会の結果がない"),
    }
}

/// 閉局要求・モード変更要求のセンター確認コードとパスワードを検査し、処理結果を返す。
fn check_comm_req(c: &CommCtl, peer: &PeerCfg, password: [u8; 6]) -> u8 {
    if c.peer != peer.own {
        PEER_CODE_ERR
    } else if c.own != peer.remote {
        OWN_CODE_ERR
    } else if c.password != password {
        PASSWORD_ERR
    } else {
        OK
    }
}

/// 二次局として1接続を処理する。
pub fn serve<S: Read + Write>(s: S, h: &mut dyn Handler, opts: &Opts) -> Result<()> {
    let mut link = Link::new(s, opts.mn);
    // S1: 開局要求
    let Msg::Comm(req) = recv_msg(&mut link, false)? else {
        return proto("開局要求を待っているのに別の電文が来た");
    };
    if req.kind != OPEN_REQ {
        return proto("最初の電文が開局要求でない");
    }
    // 要求の「相手」は自局、「当方」は要求元 (判断 D4)
    let found = h.find_peer(&req.own, &req.peer);
    let result = match &found {
        Err(code) => *code,
        Ok(p) if req.password != p.pw_open_req => PASSWORD_ERR,
        Ok(_) if req.app != 0 => APP_ERR,
        Ok(_) if req.mode > 1 => MODE_ERR,
        Ok(_) => OK,
    };
    let peer = found.ok();
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
    let mut mode = req.mode;

    // 1回の接続で、ファイルをいくつでも続けて送受信できる (マルチファイル伝送)。確定は閉局のときにまとめて行う
    let mut received: Vec<InFile> = Vec::new();
    let mut sent: Vec<OutFile> = Vec::new();
    loop {
        match recv_msg(&mut link, false)? {
            // 照会モードでは、開始要求の代わりに再送要求でも始められる (前回の照会が途中で終わったときの取り直し)
            Msg::File(f) if f.kind == START_REQ || (f.kind == RETRY_REQ && mode == 1) => {
                let check = if f.key != peer.key {
                    Err(FILE_KEY_ERR)
                } else if mode == 0 {
                    h.start_recv(&peer, &f)
                } else {
                    Ok(())
                };
                let mut rsp = FileCtl::new(START_RSP, f.name, f.key, f.rec_len, f.compress);
                rsp.aux = f.aux;
                if let Err(code) = check {
                    rsp.result = code;
                    send_fctl(&mut link, &rsp)?;
                    return rejected(code, "開始要求を拒否した");
                }
                if mode == 0 {
                    send_fctl(&mut link, &rsp)?;
                    let file = recv_data(&mut link, &f, &mut |inf| h.validate_recv(&peer, inf), opts.max_file)?;
                    received.extend(Some(file).filter(|f| !f.data.is_empty()));
                } else if sent.iter().any(|o| name_kind(&o.name) == name_kind(&f.name)) {
                    // 同じ接続で同じ種類のファイルを2度求められても、確定前の同じ内容を2度渡さない
                    rsp.result = NO_FILE;
                    send_fctl(&mut link, &rsp)?;
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
                        Ok(Some(of)) if of.rec_len != f.rec_len as usize => {
                            rsp.result = RECORD_LEN_ERR;
                            send_fctl(&mut link, &rsp)?;
                            return rejected(RECORD_LEN_ERR, "照会のレコード長が、送るファイルと違う");
                        }
                        Ok(Some(mut of)) => {
                            // 開始回答は、要求のファイル名 (サイクルを含む) と圧縮の指定をそのまま返す。
                            // Biware は、回答のファイル名が要求と違うと「全銀ファイル名の不一致」で打ち切る
                            of.name = f.name;
                            of.compress = f.compress;
                            let base = rsp.clone();
                            // 再送要求で始まったときは、開始回答を返さずにデータ電文から送る (仕様書 §VII の (2)②D)
                            if f.kind == START_REQ {
                                send_fctl(&mut link, &base)?;
                            }
                            send_data(&mut link, &base, &of, None, opts)?;
                            sent.push(of);
                        }
                    }
                }
            }
            Msg::Comm(c) if c.kind == CLOSE_REQ => {
                let result = check_comm_req(&c, &peer, peer.pw_close_req);
                let ok = result == OK;
                let rsp = CommCtl {
                    kind: CLOSE_RSP,
                    result,
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
                    for f in received.drain(..) {
                        h.commit_recv(&peer, f);
                    }
                }
                send_comm(&mut link, &rsp)?;
                if !ok {
                    return rejected(result, "閉局要求を拒否した");
                }
                // 送ったファイル (照会) は、相手が閉局回答を受け取ってから確定する。受け取れなければ、次の取得で同じものを返す
                for f in &sent {
                    h.commit_send(&peer, f);
                }
                return Ok(());
            }
            Msg::Comm(c) if c.kind == MODE_REQ => {
                // ファイルとファイルの間で、連絡と照会を切り替える (仕様書59〜62頁)
                let result = match check_comm_req(&c, &peer, peer.pw_open_req) {
                    OK if c.app != 0 => APP_ERR,
                    OK if c.mode > 1 => MODE_ERR,
                    r => r,
                };
                // ほかの回答と同じく、センター確認コードを入れ替え、残りは要求の値を返す
                let rsp = CommCtl {
                    kind: MODE_RSP,
                    result,
                    peer: c.own,
                    own: c.peer,
                    password: peer.pw_open_rsp,
                    ..c
                };
                send_comm(&mut link, &rsp)?;
                if result != OK {
                    return rejected(result, "モード変更要求を拒否した");
                }
                mode = c.mode;
            }
            _ => return proto("想定外の電文"),
        }
    }
}
