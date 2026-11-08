//! Probe: long-lived Redis SUBSCRIBE without blocking the tick or plugin events.
//! Variant selected by env `PROBE_VARIANT`: a1 (spawn_local from on_load, default), a2 (the
//! never-ending async socket future `listen_p3` awaited inside a `schedule_delayed_task` closure),
//! b (blocking std::net read loop in a scheduled task).
//! a2 and b are kept to reproduce the freeze described in the README.
use pumpkin_plugin_api_v0_2::{
    Context, EventPriority, Plugin, PluginMetadata, Result, Server,
    events::{EventData, EventHandler, PlayerChatEvent, PlayerJoinEvent},
    register_plugin,
    scheduler::{schedule_delayed_task, schedule_repeating_task},
};
use std::cell::RefCell;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use wasip3::{
    sockets::{
        ip_name_lookup::resolve_addresses,
        types::{IpAddress, IpAddressFamily, IpSocketAddress, Ipv4SocketAddress, TcpSocket},
    },
    wit_stream,
};

fn now_ms() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0)
}
fn env_or(k: &str, d: &str) -> String {
    std::env::var(k).unwrap_or_else(|_| d.to_string())
}

// ---- RESP minimal -------------------------------------------------------------------------
#[derive(Debug)]
enum Frame {
    Simple,
    Int,
    Bulk(Vec<u8>),
    Array(Vec<Frame>),
}
fn line(b: &[u8]) -> Option<(&[u8], usize)> {
    b.windows(2).position(|w| w == b"\r\n").map(|i| (&b[..i], i + 2))
}
fn parse(b: &[u8]) -> Option<(Frame, usize)> {
    let (l, n) = line(b)?;
    let rest = std::str::from_utf8(l.get(1..)?).ok()?;
    match l.first()? {
        b'+' | b'-' => Some((Frame::Simple, n)),
        b':' => rest.parse::<i64>().ok().map(|_| (Frame::Int, n)),
        b'$' => {
            let len: i64 = rest.parse().ok()?;
            if len < 0 {
                return Some((Frame::Simple, n));
            }
            let len = len as usize;
            (b.len() >= n + len + 2).then(|| (Frame::Bulk(b[n..n + len].to_vec()), n + len + 2))
        }
        b'*' => {
            let cnt: usize = rest.parse().ok()?;
            let (mut items, mut off) = (Vec::new(), n);
            for _ in 0..cnt {
                let (f, used) = parse(&b[off..])?;
                items.push(f);
                off += used;
            }
            Some((Frame::Array(items), off))
        }
        _ => None,
    }
}
fn cmd(args: &[&str]) -> Vec<u8> {
    let mut o = format!("*{}\r\n", args.len()).into_bytes();
    for a in args {
        o.extend(format!("${}\r\n{a}\r\n", a.len()).into_bytes());
    }
    o
}

// ---- Measurement state (single-threaded instance: thread_local is enough) ------------------------------
#[derive(Default)]
struct Stats {
    last: Option<Instant>,
    sum_ms: f64,
    n: u64,
    max_ms: f64,
    over200: u64,
    window_start: Option<Instant>,
    listening: bool,
    received: u64,
}
thread_local! { static ST: RefCell<Stats> = RefCell::new(Stats::default()); }

fn clock_tick() {
    let now = Instant::now();
    ST.with(|s| {
        let mut s = s.borrow_mut();
        if let Some(l) = s.last {
            let d = now.duration_since(l).as_secs_f64() * 1000.0;
            s.sum_ms += d;
            s.n += 1;
            s.max_ms = s.max_ms.max(d);
            if d > 200.0 {
                s.over200 += 1;
            }
        }
        s.last = Some(now);
        let ws = *s.window_start.get_or_insert(now);
        if now.duration_since(ws).as_secs() >= 10 {
            let avg = if s.n > 0 { s.sum_ms / s.n as f64 } else { 0.0 };
            tracing::info!(
                "CLOCK t={} avg={avg:.1}ms max={:.1}ms n={} over200={} listening={} received={}",
                now_ms(), s.max_ms, s.n, s.over200, s.listening, s.received
            );
            s.sum_ms = 0.0;
            s.n = 0;
            s.max_ms = 0.0;
            s.over200 = 0;
            s.window_start = Some(now);
        }
    });
}

fn on_msg(payload: &[u8]) {
    let recv = now_ms();
    let txt = String::from_utf8_lossy(payload);
    let sent: u128 = txt.split_whitespace().nth(1).and_then(|v| v.parse().ok()).unwrap_or(0);
    let id = txt.split_whitespace().next().unwrap_or("?").to_string();
    ST.with(|s| s.borrow_mut().received += 1);
    tracing::info!("MSG id={id} recv={recv} sent={sent} lat={}ms", recv as i128 - sent as i128);
}

fn handle_frame(f: Frame) {
    if let Frame::Array(v) = f {
        if let (Some(Frame::Bulk(k)), Some(Frame::Bulk(p))) = (v.first(), v.get(2)) {
            if k == b"message" {
                on_msg(p);
            }
        }
    }
}

// ---- Variant A: async WASI p3 socket ------------------------------------------------------
async fn listen_p3() {
    let host = env_or("REDIS_HOST", "redis");
    let port: u16 = env_or("REDIS_PORT", "6379").parse().unwrap_or(6379);
    let pass = env_or("REDIS_PASSWORD", "");
    let chan = env_or("PROBE_CHANNEL", "probe");
    let r: std::result::Result<(), String> = async {
        let ip = match resolve_addresses(host.clone()).await.map_err(|e| format!("dns {e:?}"))?
            .into_iter().find_map(|a| if let IpAddress::Ipv4(v) = a { Some(v) } else { None })
        {
            Some(v) => v,
            None => return Err("no ipv4".into()),
        };
        let sock = TcpSocket::create(IpAddressFamily::Ipv4).map_err(|e| format!("create {e:?}"))?;
        sock.connect(IpSocketAddress::Ipv4(Ipv4SocketAddress { port, address: ip }))
            .await.map_err(|e| format!("connect {e:?}"))?;
        tracing::info!("LISTEN p3 connected {host}:{port} t={}", now_ms());
        let (mut tx, rx) = wit_stream::new::<u8>();
        let _send_done = sock.send(rx);
        let (mut rd, _recv_done) = sock.receive();
        if !pass.is_empty() {
            tx.write_all(cmd(&["AUTH", &pass])).await;
        }
        tx.write_all(cmd(&["SUBSCRIBE", &chan])).await;
        ST.with(|s| s.borrow_mut().listening = true);
        let mut acc: Vec<u8> = Vec::new();
        loop {
            let (res, buf) = rd.read(Vec::with_capacity(4096)).await;
            if buf.is_empty() {
                return Err(format!("stream closed: {res:?}"));
            }
            acc.extend(buf);
            while let Some((f, used)) = parse(&acc) {
                acc.drain(..used);
                handle_frame(f);
            }
        }
    }.await;
    ST.with(|s| s.borrow_mut().listening = false);
    tracing::error!("LISTEN p3 ended: {r:?}");
}

// ---- Variant B: blocking std::net ----------------------------------------------------------
fn listen_blocking() {
    use std::io::{Read, Write};
    use std::net::{TcpStream, ToSocketAddrs};
    let host = env_or("REDIS_HOST", "redis");
    let port = env_or("REDIS_PORT", "6379");
    let chan = env_or("PROBE_CHANNEL", "probe");
    let r = (|| -> std::io::Result<()> {
        let sa = format!("{host}:{port}").to_socket_addrs()?.next().ok_or_else(|| std::io::Error::other("dns"))?;
        let mut s = TcpStream::connect(sa)?;
        s.write_all(&cmd(&["SUBSCRIBE", &chan]))?;
        tracing::info!("LISTEN std connected t={}", now_ms());
        ST.with(|s| s.borrow_mut().listening = true);
        let (mut acc, mut buf) = (Vec::new(), [0u8; 4096]);
        loop {
            let n = s.read(&mut buf)?;
            if n == 0 {
                return Ok(());
            }
            acc.extend_from_slice(&buf[..n]);
            while let Some((f, used)) = parse(&acc) {
                acc.drain(..used);
                handle_frame(f);
            }
        }
    })();
    tracing::error!("LISTEN std ended: {r:?}");
}

// ---- Events ----------------------------------------------------------------------------
struct Join;
impl EventHandler<PlayerJoinEvent> for Join {
    async fn handle(&self, _s: Server, e: EventData<PlayerJoinEvent>) -> EventData<PlayerJoinEvent> {
        tracing::info!("EVENT join t={}", now_ms());
        e
    }
}
struct Chat;
impl EventHandler<PlayerChatEvent> for Chat {
    async fn handle(&self, _s: Server, e: EventData<PlayerChatEvent>) -> EventData<PlayerChatEvent> {
        tracing::info!("EVENT chat t={}", now_ms());
        e
    }
}

struct Probe;
impl Plugin for Probe {
    fn new() -> Self {
        Probe
    }
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "async-probe".into(),
            version: "0.2.0".into(),
            authors: vec!["pumpkin-v02-probe".into()],
            description: "Async Redis subscriber probe".into(),
            dependencies: vec![],
            permissions: vec!["network.tcp.connect".into(), "network.dns".into(), "sys.env".into()],
        }
    }
    async fn on_load(&self, context: Context) -> Result<()> {
        let variant = env_or("PROBE_VARIANT", "a1");
        tracing::info!("PROBE on_load variant={variant} t={}", now_ms());
        context.register_event_handler::<PlayerJoinEvent, _>(Join, EventPriority::Normal, false);
        context.register_event_handler::<PlayerChatEvent, _>(Chat, EventPriority::Normal, false);
        schedule_repeating_task(1, 1, |_s| async { clock_tick() });
        match variant.as_str() {
            "a1" => { let _ = wit_bindgen::spawn_local(listen_p3()); }
            "a2" => {
                schedule_delayed_task(0, |_s| async { listen_p3().await });
            }
            _ => {
                schedule_delayed_task(0, |_s| async { listen_blocking() });
            }
        }
        Ok(())
    }
}
register_plugin!(Probe);
