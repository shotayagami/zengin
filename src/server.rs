//! 二次局の待ち受けと、定期処理のスレッド。銀行ノードと全銀ネットが共通で使う。

use crate::handler::Handler;
use crate::session::{Opts, Timed, serve};
use std::io;
use std::net::{SocketAddr, TcpListener};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// 同時に受ける接続の上限。超えた接続は、読まずに閉じる。
const MAX_CONNECTIONS: usize = 64;

/// 接続スレッドが終わると、同時接続数を減らす。
struct Slot(Arc<AtomicUsize>);

impl Drop for Slot {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

pub struct Server {
    pub addr: SocketAddr,
    stop: Arc<AtomicBool>,
    threads: Vec<JoinHandle<()>>,
}

impl Server {
    /// `listen` で待ち受け、接続ごとに `serve` を別スレッドで動かす。
    pub fn start<H: Handler + Send + 'static>(listen: &str, handler: Arc<Mutex<H>>, opts: Opts, who: &'static str) -> io::Result<Server> {
        let listener = TcpListener::bind(listen)?;
        listener.set_nonblocking(true)?;
        let addr = listener.local_addr()?;
        let stop = Arc::new(AtomicBool::new(false));
        let st = stop.clone();
        let active = Arc::new(AtomicUsize::new(0));
        let accept = thread::spawn(move || {
            while !st.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((s, peer)) => {
                        if active.fetch_add(1, Ordering::SeqCst) >= MAX_CONNECTIONS {
                            active.fetch_sub(1, Ordering::SeqCst);
                            eprintln!("[{who}] {peer} を断った: 同時接続が {MAX_CONNECTIONS} を超える");
                            continue;
                        }
                        let slot = Slot(active.clone());
                        let (mut h, opts) = (handler.clone(), opts.clone());
                        thread::spawn(move || {
                            let _slot = slot;
                            let timed = s.set_nonblocking(false).and_then(|_| Timed::new(s, opts.timeout, opts.max_session));
                            let Ok(timed) = timed else { return };
                            if let Err(e) = serve(timed, &mut h, &opts) {
                                eprintln!("[{who}] {peer} の接続を終了: {e}");
                            }
                        });
                    }
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => thread::sleep(Duration::from_millis(20)),
                    Err(e) => {
                        eprintln!("[{who}] accept に失敗: {e}");
                        thread::sleep(Duration::from_millis(200));
                    }
                }
            }
        });
        Ok(Server {
            addr,
            stop,
            threads: vec![accept],
        })
    }

    /// `interval` ごとに `f` を呼ぶスレッドを追加する。
    pub fn every(&mut self, interval: Duration, mut f: impl FnMut() + Send + 'static) {
        let st = self.stop.clone();
        self.threads.push(thread::spawn(move || {
            let mut waited = Duration::ZERO;
            let step = Duration::from_millis(20).min(interval);
            while !st.load(Ordering::Relaxed) {
                thread::sleep(step);
                waited += step;
                if waited >= interval {
                    waited = Duration::ZERO;
                    f();
                }
            }
        }));
    }

    pub fn stop(mut self) {
        self.stop.store(true, Ordering::Relaxed);
        for t in self.threads.drain(..) {
            let _ = t.join();
        }
    }

    /// プロセスが終わるまで待つ。
    pub fn wait(mut self) {
        for t in self.threads.drain(..) {
            let _ = t.join();
        }
    }
}
