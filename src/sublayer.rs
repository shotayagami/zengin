//! サブレイヤ: SH (8バイト)、論理ACK、連続送信 (MN/AF)。docs/spec-protocol.md §3.1。

use crate::text::MAX_TEXT;
use crate::util::{Result, proto};
use std::io::{Read, Write};

pub const SH_LEN: usize = 8;
const VERSION: u8 = 1;
const ID_INFO: u8 = 0;
const ID_ACK: u8 = 1;

#[derive(Debug, PartialEq, Eq)]
pub struct Sh {
    pub len: u16,
    pub ver: u8,
    pub id: u8,
    pub af: u8,
    pub mn: u8,
}

impl Sh {
    pub fn encode(&self) -> [u8; SH_LEN] {
        let l = self.len.to_be_bytes();
        // 上位4bit=版番号/AF、下位4bit=識別子/MN (判断 D3)
        [l[0], l[1], (self.ver << 4) | self.id, (self.af << 4) | self.mn, 0, 0, 0, 0]
    }

    pub fn decode(b: &[u8; SH_LEN]) -> Result<Sh> {
        let sh = Sh {
            len: u16::from_be_bytes([b[0], b[1]]),
            ver: b[2] >> 4,
            id: b[2] & 0xF,
            af: b[3] >> 4,
            mn: b[3] & 0xF,
        };
        if sh.ver == 0 {
            return proto("SH 版番号が 0");
        }
        match sh.id {
            ID_INFO if (sh.len as usize) < SH_LEN => proto("情報メッセージが 8 バイト未満"),
            ID_ACK if sh.len as usize != SH_LEN => proto("論理ACK の長さが 8 でない"),
            ID_INFO | ID_ACK => Ok(sh),
            x => proto(format!("SH 識別子が不正: {x}")),
        }
    }
}

/// 1本のTCP接続の上のサブレイヤ。上位とはテキスト単位でやり取りする。
pub struct Link<S> {
    s: S,
    own_mn: u8,
    peer_mn: u8,
    mn_known: bool,
    sent_noack: u8,
    recv_noack: u8,
}

impl<S: Read + Write> Link<S> {
    /// `own_mn` は自局の連続受信回数 (0〜15)。0 なら基本仕様と同じ動作。
    pub fn new(s: S, own_mn: u8) -> Self {
        Link {
            s,
            own_mn: own_mn.min(15),
            peer_mn: 0,
            mn_known: false,
            sent_noack: 0,
            recv_noack: 0,
        }
    }

    pub fn peer_mn(&self) -> u8 {
        self.peer_mn
    }

    fn read_sh(&mut self) -> Result<Sh> {
        let mut b = [0u8; SH_LEN];
        self.s.read_exact(&mut b)?;
        Sh::decode(&b)
    }

    /// テキストを1情報メッセージとして送る。`control` なら必ず論理ACKを要求して待つ。
    pub fn send(&mut self, text: &[u8], control: bool) -> Result<()> {
        if text.len() > MAX_TEXT {
            return proto("テキストが 2048 バイトを超える");
        }
        // 相手の MN 以下の件数まで AF=1 で連続送信し、次は AF=0 (§VIII)。
        let want_ack = control || self.peer_mn == 0 || self.sent_noack >= self.peer_mn;
        let sh = Sh {
            len: (SH_LEN + text.len()) as u16,
            ver: VERSION,
            id: ID_INFO,
            af: if want_ack { 0 } else { 1 },
            mn: self.own_mn,
        };
        let mut msg = Vec::with_capacity(SH_LEN + text.len());
        msg.extend_from_slice(&sh.encode());
        msg.extend_from_slice(text);
        self.s.write_all(&msg)?;
        self.s.flush()?;
        if !want_ack {
            self.sent_noack += 1;
            return Ok(());
        }
        self.sent_noack = 0;
        let ack = self.read_sh()?;
        if ack.id != ID_ACK {
            return proto("論理ACK を待っているのに情報メッセージが来た");
        }
        if !self.mn_known {
            // 開局要求に対する論理ACK で、相手の連続受信回数を知る
            self.peer_mn = ack.mn;
            self.mn_known = true;
        }
        Ok(())
    }

    /// 次の情報メッセージのテキストを受け取る。AF=0 なら論理ACKを返す。
    pub fn recv(&mut self) -> Result<Vec<u8>> {
        let sh = self.read_sh()?;
        if sh.id != ID_INFO {
            return proto("情報メッセージを待っているのに論理ACK が来た");
        }
        if !self.mn_known {
            // 開局要求の SH に相手の連続受信回数が入っている
            self.peer_mn = sh.mn;
            self.mn_known = true;
        }
        let mut text = vec![0u8; sh.len as usize - SH_LEN];
        self.s.read_exact(&mut text)?;
        if sh.af == 0 {
            self.recv_noack = 0;
            let ack = Sh {
                len: SH_LEN as u16,
                ver: VERSION,
                id: ID_ACK,
                af: 0,
                mn: self.own_mn,
            };
            self.s.write_all(&ack.encode())?;
            self.s.flush()?;
        } else {
            self.recv_noack += 1;
            if self.recv_noack > self.own_mn {
                return proto("連続受信回数を超えて論理ACK 要求なしのメッセージを受けた");
            }
        }
        Ok(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{TcpListener, TcpStream};
    use std::thread;

    #[test]
    fn sh_layout() {
        let sh = Sh {
            len: 0x0045,
            ver: 1,
            id: 0,
            af: 1,
            mn: 3,
        };
        assert_eq!(sh.encode(), [0x00, 0x45, 0x10, 0x13, 0, 0, 0, 0]);
        assert_eq!(Sh::decode(&sh.encode()).unwrap(), sh);
    }

    #[test]
    fn sh_rejects_bad() {
        assert!(Sh::decode(&[0, 8, 0x01, 0, 0, 0, 0, 0]).is_err()); // 版番号 0
        assert!(Sh::decode(&[0, 8, 0x12, 0, 0, 0, 0, 0]).is_err()); // 識別子 2
        assert!(Sh::decode(&[0, 9, 0x11, 0, 0, 0, 0, 0]).is_err()); // ACK の長さ
        assert!(Sh::decode(&[0, 7, 0x10, 0, 0, 0, 0, 0]).is_err()); // 情報が 8 未満
    }

    fn pair() -> (TcpStream, TcpStream) {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let c = TcpStream::connect(l.local_addr().unwrap()).unwrap();
        (c, l.accept().unwrap().0)
    }

    /// 送信側のMN、受信側のMNを変えて、全メッセージが届き、ACK 要求の数が規則どおりになること。
    fn run(a_mn: u8, b_mn: u8, n: usize) -> Vec<u8> {
        let (ca, cb) = pair();
        let t = thread::spawn(move || {
            let mut b = Link::new(cb, b_mn);
            let mut got = Vec::new();
            for _ in 0..=n {
                got.push(b.recv().unwrap()[0]);
            }
            got
        });
        let mut a = Link::new(ca, a_mn);
        a.send(&[0xFF], true).unwrap(); // 開局要求相当 (MN 交換)
        let sent_af: Vec<bool> = (0..n)
            .map(|i| {
                let before = a.sent_noack;
                a.send(&[i as u8], false).unwrap();
                a.sent_noack > before // AF=1 だったか
            })
            .collect();
        a.send(&[0xFE], true).ok(); // 受信側が n+1 通で終わるので失敗してよい
        let got = t.join().unwrap();
        assert_eq!(got[0], 0xFF);
        assert_eq!(&got[1..], (0..n as u8).collect::<Vec<_>>().as_slice());
        // 連続送信の上限は「相手の MN」件。その次は AF=0
        let mut run = 0u8;
        for af1 in sent_af {
            if af1 {
                run += 1;
                assert!(run <= b_mn);
            } else {
                run = 0;
            }
        }
        got
    }

    #[test]
    fn mn_combinations() {
        for (a, b) in [(0, 0), (5, 3), (3, 5), (3, 0), (0, 3), (15, 15)] {
            run(a, b, 20);
        }
    }

    #[test]
    fn receiver_rejects_too_many_noack() {
        let (ca, cb) = pair();
        let mut raw = ca;
        // MN=1 の受信側に、ACK 要求なしを 2 通続けて送る
        let t = thread::spawn(move || {
            let mut b = Link::new(cb, 1);
            let mut results = Vec::new();
            for _ in 0..3 {
                results.push(b.recv().is_ok());
            }
            results
        });
        let mk = |af: u8, v: u8| {
            let mut m = Sh {
                len: 9,
                ver: 1,
                id: 0,
                af,
                mn: 1,
            }
            .encode()
            .to_vec();
            m.push(v);
            m
        };
        raw.write_all(&mk(1, 1)).unwrap();
        raw.write_all(&mk(1, 2)).unwrap();
        raw.write_all(&mk(1, 3)).unwrap();
        assert_eq!(t.join().unwrap(), [true, false, false]);
    }
}
