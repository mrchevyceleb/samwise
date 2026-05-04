//! Tailscale-only HTTP server for serving research-task HTML reports.
//!
//! Why this exists: research tasks save a Markdown artifact to Supabase. Matt
//! needs a clickable URL on the card so he can read the report on his phone
//! or Windows box. Posting to a public Vercel URL leaks sensitive content
//! (security audits, IPs, secrets) and is search-indexable. Instead we run
//! a tiny HTTP server inside the AutoSam binary on the Mac mini, bound only
//! to the Mac mini's Tailscale IP. Anyone on Matt's tailnet can resolve it;
//! nothing on the public internet (or even his home Wi-Fi) can.
//!
//! Worker writes pre-rendered HTML to `<app_data>/reports/<task_id>.html`;
//! this server just static-serves that directory. No DB access at request
//! time, so latency is trivial and the server has no failure surface beyond
//! filesystem reads.
//!
//! Bind target: the IP returned by `tailscale ip -4`. If Tailscale isn't
//! installed or the IP can't be detected, the server doesn't start and the
//! worker simply omits `report_url` (the artifact still exists in the DB
//! and is readable through the desktop modal).

use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Html,
    routing::get,
    Router,
};

const REPORT_PORT: u16 = 8765;

static REPORT_URL_BASE: OnceLock<String> = OnceLock::new();

/// Returns `http://<tailscale-ip>:8765` once the server has bound, or None.
pub fn url_base() -> Option<&'static String> {
    REPORT_URL_BASE.get()
}

/// Best-effort: spawn the server. Logs and returns on any setup failure.
pub async fn spawn(reports_dir: PathBuf) {
    let ip = match detect_tailscale_ip().await {
        Some(ip) => ip,
        None => {
            log::warn!("[report-server] Tailscale IP not detected; report URLs disabled. Reports remain accessible via the desktop modal.");
            return;
        }
    };

    if let Err(e) = tokio::fs::create_dir_all(&reports_dir).await {
        log::warn!("[report-server] could not create reports dir {:?}: {}", reports_dir, e);
        return;
    }

    let addr = SocketAddr::new(ip, REPORT_PORT);
    let listener = match tokio::net::TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            log::warn!("[report-server] could not bind {}: {}", addr, e);
            return;
        }
    };

    let url_base = format!("http://{}", addr);
    let _ = REPORT_URL_BASE.set(url_base.clone());
    log::info!("[report-server] listening on {} (tailnet only)", url_base);

    let app = Router::new()
        .route("/r/:task_id", get(serve_report))
        .route("/health", get(|| async { "ok" }))
        .with_state(Arc::new(reports_dir));

    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            log::warn!("[report-server] serve loop exited: {}", e);
        }
    });
}

async fn detect_tailscale_ip() -> Option<IpAddr> {
    let candidates = [
        "tailscale",
        "/Applications/Tailscale.app/Contents/MacOS/Tailscale",
        "/usr/local/bin/tailscale",
        "/opt/homebrew/bin/tailscale",
    ];
    for bin in &candidates {
        let out = match tokio::process::Command::new(bin)
            .args(["ip", "-4"])
            .output()
            .await
        {
            Ok(o) => o,
            Err(_) => continue,
        };
        if !out.status.success() {
            continue;
        }
        let stdout = String::from_utf8_lossy(&out.stdout);
        for line in stdout.lines() {
            if let Ok(ip) = line.trim().parse::<IpAddr>() {
                return Some(ip);
            }
        }
    }
    None
}

async fn serve_report(
    Path(task_id): Path<String>,
    State(dir): State<Arc<PathBuf>>,
) -> Result<Html<String>, StatusCode> {
    // Hard-reject anything that isn't a UUID-shaped slug. Defense against
    // path traversal even though axum's Path extractor doesn't allow `/`.
    if task_id.is_empty()
        || task_id.len() > 64
        || !task_id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    let path = dir.join(format!("{}.html", task_id));
    match tokio::fs::read_to_string(&path).await {
        Ok(content) => Ok(Html(content)),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

/// Render Markdown to a self-contained HTML page styled to match the desktop
/// dark theme. Called by the worker after a research artifact is saved.
pub fn render_report_html(title: &str, generated_at: &str, markdown: &str) -> String {
    use pulldown_cmark::{html, Options, Parser};
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);
    opts.insert(Options::ENABLE_SMART_PUNCTUATION);
    let parser = Parser::new_ext(markdown, opts);
    let mut body = String::new();
    html::push_html(&mut body, parser);

    let escaped_title = html_escape(title);
    let escaped_generated = html_escape(generated_at);
    format!(
        r##"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="robots" content="noindex,nofollow">
<title>{title}</title>
<style>
  :root {{
    --bg: #0d1117;
    --bg-2: #161b22;
    --text: #c9d1d9;
    --text-strong: #f0f6fc;
    --muted: #8b949e;
    --accent: #6366f1;
    --accent-2: #58a6ff;
    --border: #30363d;
    --code-bg: rgba(99, 102, 241, 0.12);
  }}
  html, body {{ background: var(--bg); color: var(--text); margin: 0; }}
  body {{
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
    line-height: 1.65; font-size: 15px;
  }}
  .wrap {{ max-width: 820px; margin: 0 auto; padding: 32px 24px 80px; }}
  header {{
    border-bottom: 1px solid var(--border); padding-bottom: 18px; margin-bottom: 28px;
  }}
  header .eyebrow {{
    font-size: 11px; text-transform: uppercase; letter-spacing: 0.6px;
    color: var(--accent); font-weight: 700; margin-bottom: 6px;
  }}
  header h1 {{
    font-size: 24px; font-weight: 700; color: var(--text-strong); margin: 0;
    line-height: 1.3;
  }}
  header .meta {{ font-size: 12px; color: var(--muted); margin-top: 8px; }}
  h1, h2, h3, h4 {{ color: var(--text-strong); margin-top: 28px; margin-bottom: 10px; line-height: 1.3; }}
  h2 {{ font-size: 20px; border-bottom: 1px solid var(--border); padding-bottom: 6px; }}
  h3 {{ font-size: 17px; }}
  h4 {{ font-size: 15px; }}
  p {{ margin: 10px 0; }}
  ul, ol {{ padding-left: 24px; }}
  li {{ margin: 4px 0; }}
  a {{ color: var(--accent-2); text-decoration: underline; }}
  strong {{ color: var(--text-strong); }}
  code {{
    background: var(--code-bg); color: var(--accent);
    padding: 1px 6px; border-radius: 4px;
    font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, monospace;
    font-size: 0.9em;
  }}
  pre {{
    background: rgba(0,0,0,0.32); border: 1px solid var(--border);
    border-radius: 8px; padding: 14px 16px; overflow-x: auto; margin: 14px 0;
  }}
  pre code {{ background: transparent; color: var(--text); padding: 0; font-size: 13px; line-height: 1.55; }}
  blockquote {{
    border-left: 3px solid var(--accent); padding: 4px 14px; margin: 14px 0;
    color: var(--muted); background: rgba(99,102,241,0.05); border-radius: 0 6px 6px 0;
  }}
  table {{ border-collapse: collapse; margin: 14px 0; font-size: 13px; }}
  th, td {{ border: 1px solid var(--border); padding: 7px 12px; text-align: left; }}
  th {{ background: rgba(99, 102, 241, 0.1); color: var(--text-strong); font-weight: 700; }}
  hr {{ border: none; border-top: 1px solid var(--border); margin: 24px 0; }}
</style>
</head>
<body>
<div class="wrap">
<header>
  <div class="eyebrow">AutoSam research report</div>
  <h1>{title}</h1>
  <div class="meta">Generated {generated} · served from Mac mini over Tailscale</div>
</header>
{body}
</div>
</body>
</html>"##,
        title = escaped_title,
        generated = escaped_generated,
        body = body
    )
}

fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}
