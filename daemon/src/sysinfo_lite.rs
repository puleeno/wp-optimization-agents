use std::sync::Mutex;
use std::time::Instant;

struct CpuSample {
    at: Instant,
    cpu_secs: f64,
}

static CPU: Mutex<Option<CpuSample>> = Mutex::new(None);

fn ps_field(field: &str) -> Option<String> {
    let out = std::process::Command::new("ps")
        .args(["-o", field, "-p", &std::process::id().to_string()])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

pub fn rss_mb() -> Option<f64> {
    let raw = ps_field("rss=")?;
    // macOS/Linux `ps` reports resident set size in kilobytes
    let kb: f64 = raw.split_whitespace().next()?.trim().parse().ok()?;
    Some(kb / 1024.0)
}

fn parse_time_to_secs(s: &str) -> Option<f64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    // Formats: "SS.ss", "MM:SS.ss", "HH:MM:SS.ss" (and optional leading days "D-HH:MM:SS")
    let (days, rest) = match s.split_once('-') {
        Some((d, r)) => (d.trim().parse::<f64>().ok()?, r),
        None => (0.0, s),
    };
    let parts: Vec<&str> = rest.split(':').collect();
    let secs = match parts.len() {
        1 => parts[0].trim().parse::<f64>().ok()?,
        2 => parts[0].trim().parse::<f64>().ok()? * 60.0 + parts[1].trim().parse::<f64>().ok()?,
        3 => {
            parts[0].trim().parse::<f64>().ok()? * 3600.0
                + parts[1].trim().parse::<f64>().ok()? * 60.0
                + parts[2].trim().parse::<f64>().ok()?
        }
        _ => return None,
    };
    Some(days * 86400.0 + secs)
}

pub fn cpu_seconds() -> Option<f64> {
    let raw = ps_field("time=")?;
    parse_time_to_secs(&raw)
}

pub fn cpu_percent() -> Option<f64> {
    let now = Instant::now();
    let cur = cpu_seconds()?;
    let mut guard = CPU.lock().ok()?;
    let pct = match guard.as_ref() {
        Some(p) => {
            let dt = now.duration_since(p.at).as_secs_f64();
            if dt > 0.5 {
                let dc = cur - p.cpu_secs;
                if dc < 0.0 { 0.0 } else { (dc / dt) * 100.0 }
            } else {
                0.0
            }
        }
        None => 0.0,
    };
    *guard = Some(CpuSample { at: now, cpu_secs: cur });
    drop(guard);
    let cores = std::thread::available_parallelism().map(|n| n.get() as f64).unwrap_or(1.0);
    Some((pct / cores).clamp(0.0, 100.0))
}

pub fn worker_threads() -> u64 {
    std::thread::available_parallelism().map(|n| n.get() as u64).unwrap_or(1)
}
