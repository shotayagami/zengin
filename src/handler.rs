//! セッション層が呼ぶ、ファイルの受け渡しインタフェースと、相手ごとの認証情報。docs/spec-net.md §8。

use crate::charcode::Code;
use crate::config::Section;
use crate::control::{CenterCode, FileCtl, six};
use crate::util::{Result, lock};
use std::sync::{Arc, Mutex};

/// 相手 (センター) ごとの認証情報。
#[derive(Clone, Debug)]
pub struct PeerCfg {
    pub name: String,
    /// 自局のセンター確認コード
    pub own: CenterCode,
    /// 相手のセンター確認コード
    pub remote: CenterCode,
    pub pw_open_req: [u8; 6],
    pub pw_open_rsp: [u8; 6],
    pub pw_close_req: [u8; 6],
    pub pw_close_rsp: [u8; 6],
    pub key: [u8; 6],
}

impl PeerCfg {
    pub fn new(name: &str, own: CenterCode, remote: CenterCode, password: &str, key: &str) -> Result<PeerCfg> {
        let pw = six(password)?;
        Ok(PeerCfg {
            name: name.into(),
            own,
            remote,
            pw_open_req: pw,
            pw_open_rsp: pw,
            pw_close_req: pw,
            pw_close_rsp: pw,
            key: six(key)?,
        })
    }

    /// セクションの `center`、`password`、`key` から作る。電文区分ごとの `password_open_req` などで個別指定もできる。
    pub fn from_section(sec: &Section, own: CenterCode) -> Result<PeerCfg> {
        let mut p = PeerCfg::new(
            &sec.name,
            own,
            CenterCode::parse(sec.req("center")?)?,
            sec.req("password")?,
            sec.req("key")?,
        )?;
        for (k, slot) in [
            ("password_open_req", &mut p.pw_open_req),
            ("password_open_rsp", &mut p.pw_open_rsp),
            ("password_close_req", &mut p.pw_close_req),
            ("password_close_rsp", &mut p.pw_close_rsp),
        ] {
            if let Some(v) = sec.get(k) {
                *slot = six(v)?;
            }
        }
        Ok(p)
    }
}

/// 受信したファイル (レコードを連結したもの)。
#[derive(Clone, Debug)]
pub struct InFile {
    pub name: [u8; 12],
    pub rec_len: usize,
    pub data: Vec<u8>,
}

/// 送るファイル。
#[derive(Clone, Debug)]
pub struct OutFile {
    pub name: [u8; 12],
    pub rec_len: usize,
    pub code: Code,
    pub data: Vec<u8>,
    pub compress: bool,
}

/// 二次局が、接続ごとに呼ぶ。`Err(u8)` は相手に返す処理結果コード。
pub trait Handler {
    /// 開局要求のセンター確認コードから相手を特定する。`remote` は要求元の当方コード、`own` は要求の相手コード。
    /// 特定できなければ処理結果コードを返す: `own` が自局でない → 11 (相手センター確認コードエラー)、
    /// `remote` が登録にない → 12 (当方センター確認コードエラー)。コードの名前は、要求の項目名 (相手=受信側、当方=送信側) による。
    fn find_peer(&mut self, remote: &CenterCode, own: &CenterCode) -> std::result::Result<PeerCfg, u8>;
    /// 連絡モードの開始要求を検査する (ファイル名、レコード長)。
    fn start_recv(&mut self, peer: &PeerCfg, f: &FileCtl) -> std::result::Result<(), u8>;
    /// 連絡モードで全テキストを受けたあと、終了回答の前に呼ぶ。業務上の検査をする。
    fn validate_recv(&mut self, peer: &PeerCfg, f: &InFile) -> std::result::Result<(), u8>;
    /// ファイルを確定する。閉局回答を返す**前**に呼ぶ (返したあとに落ちると、送信側が成功と思ってファイルを捨て、データが消えるため)。
    /// 閉局回答が送信側に届かず再送されうるので、実装は重複を無視して、二重に処理しないこと。
    fn commit_recv(&mut self, peer: &PeerCfg, f: InFile);
    /// 照会モードの開始要求に対して、送るファイルを返す。ファイルがなければ `Ok(None)`。
    fn start_send(&mut self, peer: &PeerCfg, f: &FileCtl) -> std::result::Result<Option<OutFile>, u8>;
    /// 照会モードで、閉局回答まで正常に終わったあとに呼ぶ。
    fn commit_send(&mut self, peer: &PeerCfg, f: &OutFile);
}

/// 複数の接続スレッドから共有するための実装。
impl<H: Handler> Handler for Arc<Mutex<H>> {
    fn find_peer(&mut self, remote: &CenterCode, own: &CenterCode) -> std::result::Result<PeerCfg, u8> {
        lock(&**self).find_peer(remote, own)
    }
    fn start_recv(&mut self, peer: &PeerCfg, f: &FileCtl) -> std::result::Result<(), u8> {
        lock(&**self).start_recv(peer, f)
    }
    fn validate_recv(&mut self, peer: &PeerCfg, f: &InFile) -> std::result::Result<(), u8> {
        lock(&**self).validate_recv(peer, f)
    }
    fn commit_recv(&mut self, peer: &PeerCfg, f: InFile) {
        lock(&**self).commit_recv(peer, f)
    }
    fn start_send(&mut self, peer: &PeerCfg, f: &FileCtl) -> std::result::Result<Option<OutFile>, u8> {
        lock(&**self).start_send(peer, f)
    }
    fn commit_send(&mut self, peer: &PeerCfg, f: &OutFile) {
        lock(&**self).commit_send(peer, f)
    }
}
