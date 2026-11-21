//! 全銀協標準通信プロトコル (TCP/IP手順・広域IP網) と、模擬の全銀ネット/銀行ノード。
//! 仕様は `docs/spec-protocol.md` と `docs/spec-net.md`。

pub mod bank;
pub mod charcode;
pub mod config;
pub mod control;
pub mod exchange;
pub mod formats;
pub mod handler;
pub mod net;
pub mod record;
pub mod server;
pub mod session;
pub mod store;
pub mod sublayer;
pub mod text;
pub mod util;
