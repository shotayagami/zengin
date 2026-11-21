//! 伝送テキスト (TTC、制御電文の格納、データ電文の組み立てと圧縮)。docs/spec-protocol.md §3.2, §3.5。

use crate::util::{Result, proto};

/// 全銀協の仕様書のテキスト長の上限。送るテキストは、既定でこれ以下にする
pub const MAX_TEXT: usize = 2048;
/// 受け取るテキスト長の上限。市販品 (Biware) は 32,767 バイトまでのテキストを送れる (仕様書の外の拡張)
pub const MAX_TEXT_EXT: usize = 32767;
pub const TTC_LEN: usize = 5;

#[derive(Debug, PartialEq, Eq)]
pub struct Ttc {
    pub data: bool,
    pub seq: u16,
    pub len: u16,
}

/// 接続形態区分は常に 1 (PC)。判断 D7。
fn info_byte(data: bool) -> u8 {
    0x10 | data as u8
}

fn put_ttc(out: &mut Vec<u8>, data: bool, seq: u16, len: usize) {
    out.push(info_byte(data));
    out.extend_from_slice(&seq.to_be_bytes());
    out.extend_from_slice(&(len as u16).to_be_bytes());
}

pub fn parse_ttc(t: &[u8]) -> Result<Ttc> {
    if t.len() < TTC_LEN {
        return proto("テキストが TTC より短い");
    }
    let info = t[0];
    // 接続形態区分は 0/1 のどちらも受理する。電文区分は 0/1 のみ。
    if info & 0xEE != 0 {
        return proto(format!("TTC 情報区分が不正: 0x{info:02X}"));
    }
    let seq = u16::from_be_bytes([t[1], t[2]]);
    let len = u16::from_be_bytes([t[3], t[4]]);
    if len as usize != t.len() || t.len() > MAX_TEXT_EXT {
        return proto(format!("TTC テキスト長が不正: {} (実際 {})", len, t.len()));
    }
    let ttc = Ttc {
        data: info & 1 == 1,
        seq,
        len,
    };
    if !ttc.data && ttc.seq != 0 {
        return proto("制御電文のシーケンス番号が 0 でない");
    }
    Ok(ttc)
}

/// 制御電文 (64バイト) を1テキストにする。
pub fn build_control(body: &[u8]) -> Vec<u8> {
    let mut t = Vec::with_capacity(TTC_LEN + body.len());
    put_ttc(&mut t, false, 0, TTC_LEN + body.len());
    t.extend_from_slice(body);
    t
}

/// 圧縮の方式。`Some(ebcdic)` で圧縮あり。ゼロ/スペース専用の種別は EBCDIC のときだけ使う (判断 D5)。
pub type Compress = Option<bool>;

pub fn build_data(seq: u16, records: &[u8], compress: Compress) -> Vec<u8> {
    let mut t = Vec::new();
    match compress {
        None => {
            put_ttc(&mut t, true, seq, TTC_LEN + records.len());
            t.extend_from_slice(records);
        }
        Some(ebcdic) => {
            let c = compress_body(records, ebcdic);
            put_ttc(&mut t, true, seq, TTC_LEN + 2 + c.len());
            // 圧縮前のテキスト長は TTC を含む (Biware との接続で確認)
            t.extend_from_slice(&((TTC_LEN + records.len()) as u16).to_be_bytes());
            t.extend_from_slice(&c);
        }
    }
    t
}

/// データ電文のテキストから (シーケンス番号, レコード部) を取り出す。
pub fn parse_data(text: &[u8], compressed: bool) -> Result<(u16, Vec<u8>)> {
    let ttc = parse_ttc(text)?;
    if !ttc.data || ttc.seq == 0 {
        return proto("データ電文ではない");
    }
    let body = &text[TTC_LEN..];
    if !compressed {
        return Ok((ttc.seq, body.to_vec()));
    }
    if body.len() < 2 {
        return proto("圧縮テキストが短い");
    }
    let pre = u16::from_be_bytes([body[0], body[1]]) as usize;
    let Some(pre) = pre.checked_sub(TTC_LEN) else {
        return proto("圧縮前のテキスト長が TTC より短い");
    };
    Ok((ttc.seq, decompress_body(&body[2..], pre)?))
}

/// 繰り返し文字圧縮 (§X)。制御文字の上位2bit: 00=無圧縮, 01=0xF0, 10=0x40, 11=任意の1文字。末尾は 0x00。
pub fn compress_body(body: &[u8], ebcdic: bool) -> Vec<u8> {
    fn flush(out: &mut Vec<u8>, lit: &mut Vec<u8>) {
        for ch in lit.chunks(63) {
            out.push(ch.len() as u8);
            out.extend_from_slice(ch);
        }
        lit.clear();
    }
    let mut out = Vec::new();
    let mut lit = Vec::new();
    let mut i = 0;
    while i < body.len() {
        let b = body[i];
        let mut j = i;
        while j < body.len() && body[j] == b && j - i < 63 {
            j += 1;
        }
        let run = j - i;
        let special = ebcdic && (b == 0xF0 || b == 0x40);
        if special && run >= 2 {
            flush(&mut out, &mut lit);
            out.push(if b == 0xF0 { 0x40 } else { 0x80 } | run as u8);
        } else if run >= 3 {
            flush(&mut out, &mut lit);
            out.push(0xC0 | run as u8);
            out.push(b);
        } else {
            lit.extend_from_slice(&body[i..j]);
        }
        i = j;
    }
    flush(&mut out, &mut lit);
    out.push(0);
    out
}

pub fn decompress_body(data: &[u8], pre_len: usize) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(pre_len);
    let mut i = 0;
    loop {
        let Some(&c) = data.get(i) else {
            return proto("圧縮データに終了文字がない");
        };
        i += 1;
        let n = (c & 0x3F) as usize;
        match c >> 6 {
            0 if n == 0 => break,
            0 => {
                let Some(lit) = data.get(i..i + n) else {
                    return proto("圧縮データが途中で切れている");
                };
                out.extend_from_slice(lit);
                i += n;
            }
            1 | 2 if n == 0 => return proto("圧縮文字数が 0"),
            1 => out.resize(out.len() + n, 0xF0),
            2 => out.resize(out.len() + n, 0x40),
            _ => {
                let Some(&b) = data.get(i) else {
                    return proto("圧縮データが途中で切れている");
                };
                i += 1;
                if n == 0 {
                    return proto("圧縮文字数が 0");
                }
                out.resize(out.len() + n, b);
            }
        }
        if out.len() > pre_len {
            return proto("復元後の長さが圧縮前の長さを超えた");
        }
    }
    if i != data.len() || out.len() != pre_len {
        return proto("復元後の長さが圧縮前の長さと一致しない");
    }
    Ok(out)
}

/// レコード列を、レコード境界でテキストに分ける。シーケンス番号は 1 から。テキスト長は 2048 まで。
pub fn texts_for(data: &[u8], rec_len: usize, compress: Compress) -> Result<Vec<Vec<u8>>> {
    texts_for_len(data, rec_len, compress, MAX_TEXT)
}

/// `texts_for` の、テキスト長の上限を指定できる版。
pub fn texts_for_len(data: &[u8], rec_len: usize, compress: Compress, max_text: usize) -> Result<Vec<Vec<u8>>> {
    if rec_len == 0 || data.len() % rec_len != 0 {
        return proto("レコード長でデータが割り切れない");
    }
    let bounds: Vec<usize> = (0..=data.len() / rec_len).map(|i| i * rec_len).collect();
    pack(data, &bounds, compress, max_text)
}

/// 可変長レコード (各レコードの前に 2 バイトの長さ LL) の数を数え、形が正しいかを検査する。
pub fn var_records(body: &[u8]) -> Result<usize> {
    Ok(var_bounds(body)?.len() - 1)
}

/// 可変長レコードの境界 (先頭 0、各レコードの終わり)。
fn var_bounds(body: &[u8]) -> Result<Vec<usize>> {
    let mut bounds = vec![0];
    let mut i = 0;
    while i < body.len() {
        if i + 2 > body.len() {
            return proto("可変長レコードの長さ (LL) が途中で切れている");
        }
        let n = u16::from_be_bytes([body[i], body[i + 1]]) as usize;
        if n == 0 || i + 2 + n > body.len() {
            return proto("可変長レコードの長さ (LL) が不正");
        }
        i += 2 + n;
        bounds.push(i);
    }
    Ok(bounds)
}

/// 可変長レコード (LL 付きの連結) を、レコード境界でテキストに分ける。
pub fn texts_for_var(data: &[u8], compress: Compress, max_text: usize) -> Result<Vec<Vec<u8>>> {
    pack(data, &var_bounds(data)?, compress, max_text)
}

/// `bounds` (レコードの境界) で区切りながら、`max_text` に収まるだけレコードを詰めてテキストにする。
fn pack(data: &[u8], bounds: &[usize], compress: Compress, max_text: usize) -> Result<Vec<Vec<u8>>> {
    let mut texts = Vec::new();
    let mut i = 0; // bounds の添字
    while i + 1 < bounds.len() {
        // 圧縮前で収まる最大の件数から始め、圧縮して伸びた場合は減らす
        let room = max_text - TTC_LEN - if compress.is_some() { 2 } else { 0 };
        let mut j = i + 1;
        while j + 1 < bounds.len() && bounds[j + 1] - bounds[i] <= room {
            j += 1;
        }
        loop {
            if j == i {
                return proto("1レコードがテキストに収まらない");
            }
            let t = build_data((texts.len() + 1) as u16, &data[bounds[i]..bounds[j]], compress);
            if t.len() <= max_text {
                texts.push(t);
                break;
            }
            j -= 1;
        }
        i = j;
    }
    if texts.len() > u16::MAX as usize {
        return proto("テキスト数が 65535 を超える");
    }
    Ok(texts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ttc_layout() {
        let t = build_control(&[0u8; 64]);
        assert_eq!(&t[..5], [0x10, 0, 0, 0, 69]);
        let ttc = parse_ttc(&t).unwrap();
        assert_eq!(
            ttc,
            Ttc {
                data: false,
                seq: 0,
                len: 69
            }
        );
    }

    #[test]
    fn data_text_plain() {
        let t = build_data(3, &[0x41; 120], None);
        assert_eq!(&t[..5], [0x11, 0, 3, 0, 125]);
        assert_eq!(parse_data(&t, false).unwrap(), (3, vec![0x41; 120]));
    }

    #[test]
    fn ttc_rejects_bad() {
        let mut t = build_control(&[0u8; 64]);
        t[4] = 70;
        assert!(parse_ttc(&t).is_err());
        let mut t = build_control(&[0u8; 64]);
        t[2] = 1; // 制御電文のシーケンス番号が 0 でない
        assert!(parse_ttc(&t).is_err());
        assert!(parse_ttc(&[0x30, 0, 0, 0, 5]).is_err());
    }

    #[test]
    fn compress_known_bytes() {
        // 0xF0×5 → 01|5、0x40×3 → 10|3、'A'×4 → 11|4 'A'、リテラル 2 バイト → 00|2
        let body = [0xF0, 0xF0, 0xF0, 0xF0, 0xF0, 0x40, 0x40, 0x40, 0xC1, 0xC1, 0xC1, 0xC1, 0xC2, 0xC3];
        let c = compress_body(&body, true);
        assert_eq!(c, [0x45, 0x83, 0xC4, 0xC1, 0x02, 0xC2, 0xC3, 0x00]);
        assert_eq!(decompress_body(&c, body.len()).unwrap(), body);
    }

    #[test]
    fn compress_jis_uses_no_special() {
        let body = [0x30u8; 10];
        let c = compress_body(&body, false);
        assert_eq!(c, [0xCA, 0x30, 0x00]);
        // JIS でもスペース (0x20) は専用種別を使わない
        assert_eq!(compress_body(&[0x20; 5], false), [0xC5, 0x20, 0x00]);
    }

    #[test]
    fn compress_roundtrip_many() {
        let mut body = Vec::new();
        for i in 0..2000u32 {
            body.push(match i % 97 {
                0..=40 => 0x40,
                41..=60 => 0xF0,
                x => (x * 7) as u8,
            });
        }
        for ebcdic in [true, false] {
            let c = compress_body(&body, ebcdic);
            assert_eq!(decompress_body(&c, body.len()).unwrap(), body);
        }
    }

    #[test]
    fn decompress_rejects_bad() {
        assert!(decompress_body(&[0x02, 0x41], 2).is_err()); // 終了なし
        assert!(decompress_body(&[0x05, 0x41, 0x00], 5).is_err()); // 途中で切れ
        assert!(decompress_body(&[0xC3, 0x41, 0x00], 2).is_err()); // 長さ不一致
    }

    #[test]
    fn minimum_compressed_text_is_10() {
        // TTC(5)+圧縮前長(2)+制御文字(1)+1文字(1)+終了(1)=10 (日立のマニュアルの最小値と一致)
        let t = build_data(1, &[0xF1, 0xF1, 0xF1], Some(true));
        assert_eq!(t.len(), 10);
    }

    #[test]
    fn splits_at_record_boundaries() {
        let data = vec![0x40u8; 120 * 40];
        let texts = texts_for(&data, 120, None).unwrap();
        assert_eq!(texts.len(), 3); // 17+17+6
        assert_eq!(texts[0].len(), 5 + 17 * 120);
        for (i, t) in texts.iter().enumerate() {
            assert_eq!(parse_ttc(t).unwrap().seq, i as u16 + 1);
        }
        let texts200 = texts_for(&vec![0x40u8; 200 * 11], 200, None).unwrap();
        assert_eq!(texts200.len(), 2); // 10+1
    }

    #[test]
    fn compressed_texts_fit() {
        // 圧縮しにくいデータでも 2048 に収まるよう、レコード数を減らす
        let data: Vec<u8> = (0..120 * 40).map(|i| (i * 31 % 251) as u8).collect();
        let texts = texts_for(&data, 120, Some(true)).unwrap();
        let mut back = Vec::new();
        for t in &texts {
            assert!(t.len() <= MAX_TEXT);
            back.extend(parse_data(t, true).unwrap().1);
        }
        assert_eq!(back, data);
    }
}
