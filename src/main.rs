use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::Write;
use std::process::{self, Command, Stdio};

const CLK_TCK: f64 = 100.0; // USER_HZ, 100 on Linux x86_64
const RED: &str = "\x1b[1;31m";
const RESET: &str = "\x1b[0m";

struct Proc {
    pid: u32,
    time: u64,
    user: String,
    cpu: f64,
    rss: u64, // kB
    full: String,
}

fn fmt_mem(rss: u64) -> String {
    if rss >= 1_048_576 {
        format!("{:.1}gb", rss as f64 / 1_048_576.0)
    } else if rss >= 1024 {
        format!("{:.1}mb", rss as f64 / 1024.0)
    } else {
        format!("{}kb", rss)
    }
}

fn fmt_time(s: u64) -> String {
    let (h, m, sec) = (s / 3600, s % 3600 / 60, s % 60);
    if h > 0 {
        format!("{h}h {m}m {sec}s")
    } else if m > 0 {
        format!("{m}m {sec}s")
    } else {
        format!("{sec}s")
    }
}

fn term_width() -> usize {
    if let Ok(c) = env::var("COLUMNS") {
        if let Ok(n) = c.parse() {
            return n;
        }
    }
    Command::new("stty")
        .arg("size")
        .stdin(Stdio::inherit())
        .stderr(Stdio::null())
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.split_whitespace().nth(1).and_then(|c| c.parse().ok()))
        .unwrap_or(120)
}

fn load_users() -> HashMap<u32, String> {
    let mut m = HashMap::new();
    if let Ok(s) = fs::read_to_string("/etc/passwd") {
        for l in s.lines() {
            let f: Vec<&str> = l.split(':').collect();
            if f.len() > 2 {
                if let Ok(uid) = f[2].parse() {
                    m.entry(uid).or_insert_with(|| f[0].to_string());
                }
            }
        }
    }
    m
}

fn read_proc(pid: u32, uptime: f64, users: &HashMap<u32, String>) -> Option<Proc> {
    let raw = fs::read(format!("/proc/{pid}/stat")).ok()?;
    let s = String::from_utf8_lossy(&raw);
    let open = s.find('(')?;
    let close = s.rfind(')')?;
    let comm = &s[open + 1..close];
    let rest: Vec<&str> = s[close + 2..].split_whitespace().collect();
    let ut: u64 = rest.get(11)?.parse().ok()?;
    let st: u64 = rest.get(12)?.parse().ok()?;
    let start: u64 = rest.get(19)?.parse().ok()?;

    let ticks = (ut + st) as f64;
    let elapsed = uptime - start as f64 / CLK_TCK;
    let cpu = if elapsed > 0.0 { 100.0 * ticks / CLK_TCK / elapsed } else { 0.0 };

    let status = fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    let mut uid = 0u32;
    let mut rss = 0u64;
    for l in status.lines() {
        if let Some(v) = l.strip_prefix("Uid:") {
            uid = v.split_whitespace().nth(1)?.parse().ok()?; // effective
        } else if let Some(v) = l.strip_prefix("VmRSS:") {
            rss = v.split_whitespace().next()?.parse().ok()?;
        }
    }
    let user = users.get(&uid).cloned().unwrap_or_else(|| uid.to_string());

    let cmdline = fs::read(format!("/proc/{pid}/cmdline")).unwrap_or_default();
    let args: Vec<String> = cmdline
        .split(|&b| b == 0)
        .filter(|a| !a.is_empty())
        .map(|a| String::from_utf8_lossy(a).into_owned())
        .collect();
    let full = if args.is_empty() { format!("[{comm}]") } else { args.join(" ") };

    Some(Proc { pid, time: (ticks / CLK_TCK) as u64, user, cpu, rss, full })
}

fn simplify(full: &str) -> String {
    let mut it = full.split_whitespace();
    let mut d = it.next().unwrap_or("").to_string();
    if let Some(second) = it.next() {
        if !second.starts_with('-') {
            d.push(' ');
            d.push_str(second);
        }
    }
    d
}

fn wrap(s: &str, w: usize) -> Vec<String> {
    let c: Vec<char> = s.chars().collect();
    let mut out = Vec::new();
    let mut start = 0;
    while c.len() - start > w {
        let end = start + w;
        let mut brk = end;
        if c[end] != ' ' {
            if let Some(p) = c[start..end].iter().rposition(|&x| x == ' ') {
                if p > 0 {
                    brk = start + p + 1;
                }
            }
        }
        out.push(c[start..brk].iter().collect::<String>().trim_end().to_string());
        start = brk;
        while start < c.len() && c[start] == ' ' {
            start += 1;
        }
    }
    out.push(c[start..].iter().collect());
    out
}

fn highlight(line: &str, kw: &str) -> String {
    if kw.is_empty() {
        return line.to_string();
    }
    let hl = line.to_ascii_lowercase();
    let k = kw.to_ascii_lowercase();
    let mut out = String::new();
    let mut i = 0;
    while let Some(p) = hl[i..].find(&k) {
        out.push_str(&line[i..i + p]);
        out.push_str(RED);
        out.push_str(&line[i + p..i + p + k.len()]);
        out.push_str(RESET);
        i += p + k.len();
    }
    out.push_str(&line[i..]);
    out
}

fn json_str(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for ch in s.chars() {
        match ch {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn die(msg: &str) -> ! {
    eprintln!("{msg}\nusage: psf [-a] [-j] [-u user] [-s<p|m|c|t><a|d>] [keyword]");
    process::exit(2);
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut all = false;
    let mut json = false;
    let mut keyword = String::new();
    let mut user_f: Option<String> = None;
    let (mut sf, mut sd) = ('c', 'd');

    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        if a == "-a" {
            all = true;
        } else if a == "-j" {
            json = true;
        } else if a == "-u" {
            i += 1;
            user_f = Some(args.get(i).unwrap_or_else(|| die("-u needs a user")).clone());
        } else if a.starts_with("-u") && a.len() > 2 {
            user_f = Some(a[2..].to_string());
        } else if a.starts_with("-s") && a.len() == 4 {
            let b: Vec<char> = a[2..].chars().collect();
            if !"pmct".contains(b[0]) || !"ad".contains(b[1]) {
                die("bad -s spec");
            }
            sf = b[0];
            sd = b[1];
        } else {
            keyword = a.to_string();
        }
        i += 1;
    }

    let users = load_users();
    let uptime: f64 = fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|s| s.split_whitespace().next().and_then(|v| v.parse().ok()))
        .unwrap_or(0.0);
    let kw_lc = keyword.to_ascii_lowercase();

    let mut procs: Vec<Proc> = fs::read_dir("/proc")
        .expect("read /proc")
        .filter_map(|e| e.ok()?.file_name().to_str()?.parse::<u32>().ok())
        .filter_map(|pid| read_proc(pid, uptime, &users))
        .filter(|p| all || !(p.cpu < 0.05 && p.rss == 0))
        .filter(|p| user_f.as_deref().map_or(true, |u| p.user == u))
        .filter(|p| kw_lc.is_empty() || p.full.to_ascii_lowercase().contains(&kw_lc))
        .collect();

    procs.sort_by(|a, b| {
        let o = match sf {
            'p' => a.pid.cmp(&b.pid),
            'm' => a.rss.cmp(&b.rss),
            't' => a.time.cmp(&b.time),
            _ => a.cpu.partial_cmp(&b.cpu).unwrap_or(std::cmp::Ordering::Equal),
        };
        (if sd == 'd' { o.reverse() } else { o }).then(a.pid.cmp(&b.pid))
    });

    let mut out = String::new();

    if json {
        out.push_str("[\n");
        for (n, p) in procs.iter().enumerate() {
            out.push_str(&format!(
                "  {{\"pid\":{},\"time_seconds\":{},\"time\":{},\"user\":{},\"cpu\":{:.1},\"mem_kb\":{},\"mem\":{},\"command\":{}}}{}\n",
                p.pid,
                p.time,
                json_str(&fmt_time(p.time)),
                json_str(&p.user),
                p.cpu,
                p.rss,
                json_str(&fmt_mem(p.rss)),
                json_str(&p.full),
                if n + 1 < procs.len() { "," } else { "" }
            ));
        }
        out.push_str("]\n");
        let _ = std::io::stdout().lock().write_all(out.as_bytes());
        return;
    }

    // Pre-format rows, compute column widths (2-space gap between all columns)
    let rows: Vec<[String; 5]> = procs
        .iter()
        .map(|p| {
            [
                p.pid.to_string(),
                fmt_time(p.time),
                p.user.clone(),
                format!("{:.1}%", p.cpu),
                fmt_mem(p.rss),
            ]
        })
        .collect();
    let hdr = ["PID", "TIME", "USER", "CPU", "MEM"];
    let mut w = [0usize; 5];
    for c in 0..5 {
        w[c] = hdr[c].len();
        for r in &rows {
            w[c] = w[c].max(r[c].chars().count());
        }
    }
    let mk_prefix = |cols: &[&str; 5]| -> String {
        let mut s = String::new();
        for c in 0..5 {
            s.push_str(&format!("{:<width$}  ", cols[c], width = w[c]));
        }
        s
    };

    let width = term_width();
    out.push_str(&mk_prefix(&hdr));
    out.push_str("COMMAND\n");

    for (p, r) in procs.iter().zip(rows.iter()) {
        let prefix = mk_prefix(&[&r[0], &r[1], &r[2], &r[3], &r[4]]);
        let indent = prefix.chars().count();
        let cw = width.saturating_sub(indent).max(20);
        let display = if all { p.full.clone() } else { simplify(&p.full) };
        for (n, line) in wrap(&display, cw).iter().enumerate() {
            if n == 0 {
                out.push_str(&prefix);
            } else {
                out.push_str(&" ".repeat(indent));
            }
            out.push_str(&highlight(line, &keyword));
            out.push('\n');
        }
    }
    let _ = std::io::stdout().lock().write_all(out.as_bytes());
}
