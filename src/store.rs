//! 状態ファイル (1行1レコードのテキスト)。`dir` が None ならメモリ上だけで動く (試験用)。

use crate::util::{Clock, Result, ZError, proto};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Store {
    dir: Option<PathBuf>,
}

impl Store {
    pub fn new(dir: Option<PathBuf>) -> Result<Store> {
        if let Some(d) = &dir {
            fs::create_dir_all(d).map_err(ZError::Io)?;
        }
        Ok(Store { dir })
    }

    pub fn is_persistent(&self) -> bool {
        self.dir.is_some()
    }

    /// ファイルがなければ `Ok(None)`。あるのに読めない (UTF-8 でない、権限がない) 場合は Err。
    pub fn read(&self, name: &str) -> Result<Option<Vec<String>>> {
        let Some(d) = &self.dir else { return Ok(None) };
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

    /// CLI が置いた依頼のファイルを、読んで取り出す。
    /// 別名に移してから読むので、読んだあとに CLI が追記しても、その行は失われず、次の周期で取り出される。
    /// 処理が終わったら `done` を呼ぶ。途中で落ちた場合は、次回、続きから取り出す。
    pub fn take(&self, name: &str) -> Result<Option<Vec<String>>> {
        let Some(d) = &self.dir else { return Ok(None) };
        let taking = format!("{name}.taking");
        if !d.join(&taking).exists() {
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
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(d.join(name)) {
            let _ = writeln!(f, "{line}");
        }
    }

    pub fn remove(&self, name: &str) {
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
