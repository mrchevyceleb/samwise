//! Which agent harness writes the code.
//!
//! Sam's coding runs used to be hardwired to the Claude Code CLI. Claude Code
//! validates model names client-side and only accepts Anthropic IDs, so running
//! any other model meant standing a LiteLLM proxy in front of it to accept
//! `claude-opus-4-8` and rewrite it to a Fireworks model. That translation layer
//! costs real capability: `drop_params: true` is mandatory (it silently discards
//! whatever the target provider won't take), and a text-only target rejects
//! image blocks outright, which is how Sam went blind on screenshots.
//!
//! pi talks to providers natively — no proxy, no rewriting, no dropped params,
//! and vision works wherever the chosen model supports it. This module picks
//! between the two harnesses so the swap is one env var, and rolling back is
//! the same.
//!
//! Everything around the harness (worktrees, board updates, review cycles,
//! deploys, Slack/Telegram ingestion) is harness-agnostic and untouched.

/// The agent harness used for a coding run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoderBackend {
    /// Anthropic's Claude Code CLI. Non-Anthropic models require the LiteLLM proxy.
    ClaudeCode,
    /// pi headless (`pi -p --mode json`). Any provider/model pi is configured for.
    Pi,
}

impl CoderBackend {
    pub fn as_str(self) -> &'static str {
        match self {
            CoderBackend::ClaudeCode => "claude-code",
            CoderBackend::Pi => "pi",
        }
    }
}

/// Parse a backend name. Unknown values fall back to Claude Code rather than
/// failing the run: a typo in the unit file should not take Sam offline.
fn parse_backend(raw: &str) -> Option<CoderBackend> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "" => None,
        "claude" | "claude-code" | "claudecode" => Some(CoderBackend::ClaudeCode),
        "pi" => Some(CoderBackend::Pi),
        other => {
            log::warn!(
                "[coder] unknown AUTOSAM_CODER value {:?}; falling back to claude-code",
                other
            );
            None
        }
    }
}

/// Which harness this process should use. `AUTOSAM_CODER=claude|pi`,
/// defaulting to Claude Code so the swap is opt-in.
pub fn active_backend() -> CoderBackend {
    std::env::var("AUTOSAM_CODER")
        .ok()
        .and_then(|v| parse_backend(&v))
        .unwrap_or(CoderBackend::ClaudeCode)
}

// ── pi configuration ─────────────────────────────────────────────────

/// Model pi runs when nothing overrides it. kimi-k3 reports `images: yes` in
/// `pi --list-models`, so screenshots reach the model as pixels instead of
/// going through a local describe pass.
pub const PI_DEFAULT_PROVIDER: &str = "fireworks";
pub const PI_DEFAULT_MODEL: &str = "accounts/fireworks/routers/kimi-k3";
pub const PI_DEFAULT_THINKING: &str = "high";

/// Resolved pi invocation settings.
pub struct PiConfig {
    pub provider: String,
    pub model: String,
    pub thinking: String,
}

fn env_non_empty(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

impl PiConfig {
    /// Read from env, falling back to the constants above.
    ///
    /// `model_override` is the per-call model the worker already threads through
    /// `run_claude_code_*`. It may be a bare model id or a `provider/model` pair.
    pub fn resolve(model_override: Option<&str>) -> Self {
        let mut provider = env_non_empty("AUTOSAM_PI_PROVIDER")
            .unwrap_or_else(|| PI_DEFAULT_PROVIDER.to_string());
        let mut model =
            env_non_empty("AUTOSAM_PI_MODEL").unwrap_or_else(|| PI_DEFAULT_MODEL.to_string());

        if let Some(raw) = model_override.map(str::trim).filter(|s| !s.is_empty()) {
            // Callers pass Anthropic-shaped names (e.g. "claude-opus-4-8") that
            // only ever meant "whatever Sam codes with". Those are meaningless
            // to pi, so ignore them and keep the configured model. An explicit
            // "provider/model" override is honoured.
            if let Some((p, m)) = split_provider_model(raw) {
                provider = p;
                model = m;
            } else if !raw.starts_with("claude-") {
                model = raw.to_string();
            }
        }

        PiConfig {
            provider,
            model,
            thinking: env_non_empty("AUTOSAM_PI_THINKING")
                .unwrap_or_else(|| PI_DEFAULT_THINKING.to_string()),
        }
    }
}

/// Split a `provider/model` override. Fireworks ids are themselves slash-heavy
/// (`accounts/fireworks/routers/kimi-k3`), so only treat the first segment as a
/// provider when the remainder still looks like a model id.
fn split_provider_model(raw: &str) -> Option<(String, String)> {
    let (head, rest) = raw.split_once('/')?;
    if head.is_empty() || rest.is_empty() || head.contains(' ') {
        return None;
    }
    // "accounts/..." is a Fireworks model id, not provider/model.
    if head == "accounts" {
        return None;
    }
    Some((head.to_string(), rest.to_string()))
}

/// Model families that cannot accept image blocks, per the `images` column of
/// `pi --list-models`. Matched as substrings against the configured model id.
const TEXT_ONLY_MODEL_MARKERS: [&str; 5] =
    ["glm", "deepseek", "minimax", "gpt-oss", "nemotron"];

/// Does this model id accept image input? Conservative: anything matching a
/// known text-only family is treated as blind.
fn model_is_vision_capable(model: &str) -> bool {
    let lowered = model.to_ascii_lowercase();
    !TEXT_ONLY_MODEL_MARKERS
        .iter()
        .any(|marker| lowered.contains(marker))
}

/// Can the coding model read an image directly?
///
/// This used to be a hand-maintained env flag, and it drifted: the flag was set
/// for vision-capable Kimi K3 on 2026-07-23, the model was reverted to text-only
/// GLM 5.2 the next day, and the flag stayed on for 13 days. Sam kept working on
/// screenshots he could not see. Deriving the answer from the harness and model
/// actually in use removes that failure mode.
///
/// `AUTOSAM_CODER_HANDLES_VISION` still wins when set explicitly, so there is an
/// escape hatch for a backend this cannot infer (notably the Claude Code path,
/// where the real model is whatever the LiteLLM proxy rewrites to).
pub fn coder_handles_vision() -> bool {
    if let Some(raw) = env_non_empty("AUTOSAM_CODER_HANDLES_VISION") {
        return matches!(
            raw.to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        );
    }
    match active_backend() {
        // pi sends images to the provider natively, so this is purely a
        // question of whether the chosen model accepts them.
        CoderBackend::Pi => model_is_vision_capable(&PiConfig::resolve(None).model),
        // The proxy hides the real model behind an Anthropic alias, so assume
        // it cannot see and let the local describe pass run. Worst case that is
        // redundant work; the other way round is silent blindness.
        CoderBackend::ClaudeCode => false,
    }
}

/// Returns (executable, prefix_args) for spawning pi headless.
///
/// Order matters. `/usr/bin/pi` on this host is an unrelated 2024 ELF binary (a
/// Lisp), so a bare PATH lookup can silently launch the wrong program. Resolve
/// the real Node entrypoint explicitly and only fall back to PATH last.
pub fn find_pi_command() -> (String, Vec<String>) {
    // 1. Explicit override wins: "node /path/to/cli.js" or a single binary path.
    if let Some(raw) = env_non_empty("AUTOSAM_PI_CLI") {
        let mut parts = raw.split_whitespace().map(str::to_string);
        if let Some(exe) = parts.next() {
            return (exe, parts.collect());
        }
    }

    // 2. The installed pi package entrypoint, run under node directly.
    if let Some(path) = env_non_empty("AUTOSAM_PI_CLI_PATH") {
        if std::path::Path::new(&path).exists() {
            return ("node".to_string(), vec![path]);
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        let cli_js = format!(
            "{}/node_modules/@earendil-works/pi-coding-agent/dist/cli.js",
            home
        );
        if std::path::Path::new(&cli_js).exists() {
            return ("node".to_string(), vec![cli_js]);
        }
        // 3. The bun shim, which symlinks to the same cli.js.
        let bun_bin = format!("{}/.bun/bin/pi", home);
        if std::path::Path::new(&bun_bin).exists() {
            return ("node".to_string(), vec![bun_bin]);
        }
    }

    // 4. Last resort. May hit the wrong `pi` — logged so it is diagnosable.
    log::warn!(
        "[coder] no pi entrypoint found in HOME; falling back to `pi` on PATH \
         (note: /usr/bin/pi on this host is an unrelated binary)"
    );
    ("pi".to_string(), Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_parsing_accepts_known_aliases() {
        assert_eq!(parse_backend("pi"), Some(CoderBackend::Pi));
        assert_eq!(parse_backend("PI"), Some(CoderBackend::Pi));
        assert_eq!(parse_backend("claude"), Some(CoderBackend::ClaudeCode));
        assert_eq!(parse_backend("claude-code"), Some(CoderBackend::ClaudeCode));
    }

    #[test]
    fn unknown_or_empty_backend_falls_back_to_claude() {
        assert_eq!(parse_backend(""), None);
        assert_eq!(parse_backend("gpt"), None);
    }

    #[test]
    fn anthropic_shaped_overrides_do_not_leak_into_pi() {
        // worker.rs passes "claude-opus-4-8" to mean "Sam's coding model".
        // Handing that to pi would request a model the provider does not have.
        let cfg = PiConfig::resolve(Some("claude-opus-4-8"));
        assert_eq!(cfg.model, PI_DEFAULT_MODEL);
        assert_eq!(cfg.provider, PI_DEFAULT_PROVIDER);
    }

    #[test]
    fn explicit_provider_model_override_is_honoured() {
        let cfg = PiConfig::resolve(Some("anthropic/claude-sonnet-5"));
        assert_eq!(cfg.provider, "anthropic");
        assert_eq!(cfg.model, "claude-sonnet-5");
    }

    #[test]
    fn fireworks_style_ids_are_not_split_as_provider() {
        assert_eq!(split_provider_model("accounts/fireworks/routers/kimi-k3"), None);
        let cfg = PiConfig::resolve(Some("accounts/fireworks/routers/kimi-k3"));
        assert_eq!(cfg.model, "accounts/fireworks/routers/kimi-k3");
        assert_eq!(cfg.provider, PI_DEFAULT_PROVIDER);
    }

    /// The exact drift that blinded Sam for 13 days: kimi-k3 sees images,
    /// glm-5p2 does not, and the answer must follow the model rather than a
    /// flag someone remembered to flip.
    #[test]
    fn vision_capability_follows_the_model() {
        assert!(model_is_vision_capable("accounts/fireworks/routers/kimi-k3"));
        assert!(model_is_vision_capable("accounts/fireworks/routers/kimi-k3-fast"));
        assert!(model_is_vision_capable("claude-sonnet-5"));
        assert!(!model_is_vision_capable("accounts/fireworks/models/glm-5p2"));
        assert!(!model_is_vision_capable("accounts/fireworks/models/deepseek-v4-pro"));
        assert!(!model_is_vision_capable("accounts/fireworks/models/gpt-oss-120b"));
    }

    #[test]
    fn vision_check_is_case_insensitive() {
        assert!(!model_is_vision_capable("Accounts/Fireworks/Models/GLM-5P2"));
    }

    #[test]
    fn empty_override_keeps_defaults() {
        let cfg = PiConfig::resolve(Some("   "));
        assert_eq!(cfg.model, PI_DEFAULT_MODEL);
        let cfg = PiConfig::resolve(None);
        assert_eq!(cfg.model, PI_DEFAULT_MODEL);
    }
}
