//! 状態ファイル (1行1レコードのテキスト)。`dir` が None ならメモリ上だけで動く (試験用)。
//!
//! ノードの状態 (口座、送信待ち、重複の記録など) は、`write_snapshot` で1つのファイル (`state.snapshot`) にまとめて書く。
//! 一時ファイルに書いてから1回の名前の変更で置き換えるので、途中で落ちても「全部書けた」か「何も書いていない」の
//! どちらかになる (状態をいくつものファイルに分けて書くと、一部だけ書けた状態が残り、再送で二重に処理しうるため)。
//! 読むときは、スナップショットにあればそれを、なければ同じ名前の個別のファイルを読む (古い形式からの移行と、
//! 依頼のファイルなどスナップショットに入れないもののため)。

use crate::util::{Clock, Result, ZError, proto};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// ノードの状態をまとめて書くファイル。
const SNAPSHOT: &str = "state.snapshot";
/// スナップショットの中の、部分の見出しの印 (データの行の先頭には現れない制御文字)。
const SNAPSHOT_MARK: char = '\u{1}';

#[derive(Clone, Debug)]
pub struct Store {
    dir: Option<PathBuf>,
    /// 試験用: 残りの書き込みの回数。0 になったら、以降の書き込みを黙って捨てる (プロセスがそこで落ちたのと同じ)
    budget: Option<Arc<AtomicUsize>>,
}

impl Store {
    pub fn new(dir: Option<PathBuf>) -> Result<Store> {
        if let Some(d) = &dir {
            fs::create_dir_all(d).map_err(ZError::Io)?;
        }
        Ok(Store { dir, budget: None })
    }

    /// 試験用: `n` 回書き込んだあとの書き込み (ファイルの置き換え、追記、削除) を、すべて捨てる保存先を返す。
    /// 状態をいくつものファイルに書く途中でプロセスが落ちた場面を、落ちる位置ごとに再現するために使う。
    pub fn crash_after(&self, n: usize) -> Store {
        Store {
            dir: self.dir.clone(),
            budget: Some(Arc::new(AtomicUsize::new(n))),
        }
    }

    /// 試験用: 残りの書き込みの回数 (`crash_after` でなければ None)。
    pub fn budget_left(&self) -> Option<usize> {
        self.budget.as_ref().map(|b| b.load(Ordering::SeqCst))
    }

    /// 書き込んでよいか (試験用の回数制限を1つ使う)。
    fn may_write(&self) -> bool {
        self.budget
            .as_ref()
            .is_none_or(|b| b.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_sub(1)).is_ok())
    }

    pub fn is_persistent(&self) -> bool {
        self.dir.is_some()
    }

    /// ファイルがなければ `Ok(None)`。あるのに読めない (UTF-8 でない、権限がない) 場合は Err。
    /// スナップショットに同じ名前の部分があれば、そちらを返す。
    pub fn read(&self, name: &str) -> Result<Option<Vec<String>>> {
        let Some(d) = &self.dir else { return Ok(None) };
        if let Some(part) = self.snapshot_part(name)? {
            return Ok(Some(part));
        }
        match fs::read_to_string(d.join(name)) {
            Ok(text) => Ok(Some(text.lines().map(str::to_string).collect())),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => proto(format!("状態ファイル {name} を読めない: {e}")),
        }
    }

    /// ファイルがなければ空。状態ファイルの読み込み用。
    pub fn lines(&self, name: &str) -> Result<Vec<String>> {
        Ok(self.read(name)?.unwrap_or_default())
    }

    /// 一時ファイルに書いて、ディスクへ書き出してから置き換える。
    pub fn write(&self, name: &str, lines: &[String]) -> Result<()> {
        let Some(d) = &self.dir else { return Ok(()) };
        if !self.may_write() {
            return Ok(());
        }
        let tmp = d.join(format!("{name}.tmp"));
        let mut body = lines.join("\n");
        if !body.is_empty() {
            body.push('\n');
        }
        let mut f = fs::File::create(&tmp).map_err(ZError::Io)?;
        f.write_all(body.as_bytes()).map_err(ZError::Io)?;
        f.sync_all().map_err(ZError::Io)?;
        drop(f);
        fs::rename(&tmp, d.join(name)).map_err(ZError::Io)?;
        Ok(())
    }

    /// ノードの状態を、名前ごとの行の組として、1つのファイルにまとめて置き換える (原子的)。
    pub fn write_snapshot(&self, parts: &[(&str, Vec<String>)]) -> Result<()> {
        let mut lines = Vec::new();
        for (name, part) in parts {
            if name.contains(['\t', '\n']) {
                return proto(format!("スナップショットの部分の名前が不正: {name:?}"));
            }
            lines.push(format!("{SNAPSHOT_MARK}{name}\t{}", part.len()));
            lines.extend(part.iter().map(|l| l.replace('\n', " ")));
        }
        self.write(SNAPSHOT, &lines)
    }

    /// スナップショットの、`name` の部分。スナップショットがないか、その部分がなければ None。
    fn snapshot_part(&self, name: &str) -> Result<Option<Vec<String>>> {
        let Some(d) = &self.dir else { return Ok(None) };
        let text = match fs::read_to_string(d.join(SNAPSHOT)) {
            Ok(t) => t,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return proto(format!("状態ファイル {SNAPSHOT} を読めない: {e}")),
        };
        let mut lines = text.lines();
        while let Some(head) = lines.next() {
            let parsed = head
                .strip_prefix(SNAPSHOT_MARK)
                .and_then(|h| h.split_once('\t'))
                .and_then(|(n, c)| Some((n, c.parse::<usize>().ok()?)));
            let Some((part, count)) = parsed else {
                return proto(format!("状態ファイル {SNAPSHOT} が壊れている: {head:?}"));
            };
            let body: Vec<String> = lines.by_ref().take(count).map(str::to_string).collect();
            if body.len() != count {
                return proto(format!("状態ファイル {SNAPSHOT} が途中で切れている ({part})"));
            }
            if part == name {
                return Ok(Some(body));
            }
        }
        Ok(None)
    }

    /// CLI が置いた依頼のファイルを、読んで取り出す。
    /// 別名に移してから読むので、読んだあとに CLI が追記しても、その行は失われず、次の周期で取り出される。
    /// 処理が終わったら `done` を呼ぶ。途中で落ちた場合は、次回、続きから取り出す。
    pub fn take(&self, name: &str) -> Result<Option<Vec<String>>> {
        let Some(d) = &self.dir else { return Ok(None) };
        let taking = format!("{name}.taking");
        if !d.join(&taking).exists() {
            if !self.may_write() {
                return Ok(None);
            }
            match fs::rename(d.join(name), d.join(&taking)) {
                Ok(()) => {}
                Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
                Err(e) => return proto(format!("{name} を取り出せない: {e}")),
            }
        }
        self.read(&taking)
    }

    pub fn done(&self, name: &str) {
        self.remove(&format!("{name}.taking"));
    }

    pub fn append(&self, name: &str, line: &str) {
        let Some(d) = &self.dir else { return };
        if !self.may_write() {
            return;
        }
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(d.join(name)) {
            let _ = writeln!(f, "{line}");
        }
    }

    pub fn remove(&self, name: &str) {
        if !self.may_write() {
            return;
        }
        if let Some(d) = &self.dir {
            let _ = fs::remove_file(d.join(name));
        }
    }

    /// 日時つきの記録を `journal.tsv` に追記し、標準エラーにも出す。
    pub fn journal(&self, clock: &Clock, who: &str, msg: &str) {
        let t = clock.now();
        let line = format!("{} {}\t{who}\t{msg}", t.ymd(), t.hms());
        eprintln!("[{who}] {msg}");
        self.append("journal.tsv", &line);
    }
}
