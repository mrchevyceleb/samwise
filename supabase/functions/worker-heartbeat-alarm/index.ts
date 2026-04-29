// worker-heartbeat-alarm
//
// Pages Matt on Telegram when Sam's worker stops heartbeating. Closes the
// "silent death" gap: if Sam panics past the supervisor, deadlocks, or the
// whole Mac sleeps, Matt finds out within ~5 minutes instead of next morning.
//
// Schedule: pg_cron, every 2 minutes.
// Auth: --no-verify-jwt (called by pg_net from inside the project).
// Env:  SUPABASE_URL, SUPABASE_SERVICE_ROLE_KEY (auto-injected),
//       TELEGRAM_BOT_TOKEN, TELEGRAM_CHAT_ID (set via Supabase secrets).

import { createClient } from "https://esm.sh/@supabase/supabase-js@2";

const SUPABASE_URL = Deno.env.get("SUPABASE_URL")!;
const SUPABASE_SERVICE_KEY = Deno.env.get("SUPABASE_SERVICE_ROLE_KEY")!;
const TG_TOKEN = Deno.env.get("TELEGRAM_BOT_TOKEN");
const TG_CHAT_ID = Deno.env.get("TELEGRAM_CHAT_ID");

// Worker is "stale" after this much silence. Tick is ~5s, so anything past
// 3 min means at least ~36 missed ticks - definitely not normal lag.
const STALE_THRESHOLD_SEC = 180;
// While stale, re-page no more often than this. Prevents alert spam if Sam's
// down for hours; first ping is immediate, follow-ups every 10 min.
const REALERT_INTERVAL_SEC = 600;

const sb = createClient(SUPABASE_URL, SUPABASE_SERVICE_KEY);

interface WorkerRow {
  id: string;
  machine_name: string;
  status: string;
  last_heartbeat: string | null;
  last_stale_alert_at: string | null;
}

async function pageTelegram(text: string): Promise<boolean> {
  if (!TG_TOKEN || !TG_CHAT_ID) return false;
  const r = await fetch(
    `https://api.telegram.org/bot${TG_TOKEN}/sendMessage`,
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ chat_id: TG_CHAT_ID, text }),
    },
  );
  if (!r.ok) {
    console.error(`[heartbeat-alarm] telegram ${r.status}: ${await r.text()}`);
    return false;
  }
  return true;
}

Deno.serve(async () => {
  if (!TG_TOKEN || !TG_CHAT_ID) {
    return new Response(
      JSON.stringify({ error: "TELEGRAM_BOT_TOKEN / TELEGRAM_CHAT_ID not set" }),
      { status: 500, headers: { "Content-Type": "application/json" } },
    );
  }

  const { data, error } = await sb
    .from("ae_workers")
    .select("id, machine_name, status, last_heartbeat, last_stale_alert_at")
    .eq("status", "online");

  if (error) {
    return new Response(
      JSON.stringify({ error: `db: ${error.message}` }),
      { status: 500, headers: { "Content-Type": "application/json" } },
    );
  }

  const now = Date.now();
  const alerted: string[] = [];
  const recovered: string[] = [];

  for (const w of (data ?? []) as WorkerRow[]) {
    if (!w.last_heartbeat) continue;
    const hbAgeSec = (now - new Date(w.last_heartbeat).getTime()) / 1000;
    const isStale = hbAgeSec > STALE_THRESHOLD_SEC;

    if (isStale) {
      const lastAlertMs = w.last_stale_alert_at
        ? new Date(w.last_stale_alert_at).getTime()
        : 0;
      const sinceLastSec = (now - lastAlertMs) / 1000;
      const shouldAlert = lastAlertMs === 0 || sinceLastSec > REALERT_INTERVAL_SEC;

      if (shouldAlert) {
        const mins = Math.round(hbAgeSec / 60);
        const msg =
          `🚨 Sam silent for ${mins}m on '${w.machine_name}'. ` +
          `Worker thread is dead - tickets are NOT getting picked up. ` +
          `Open Samwise; if no Codex review is mid-flight, click Stop+Start to relaunch the loop.`;
        const ok = await pageTelegram(msg);
        if (ok) {
          await sb
            .from("ae_workers")
            .update({ last_stale_alert_at: new Date().toISOString() })
            .eq("id", w.id);
          alerted.push(w.machine_name);
        }
      }
    } else if (w.last_stale_alert_at !== null) {
      // Worker came back. Page once that he's recovered, then clear the flag.
      await pageTelegram(
        `✅ Sam back online on '${w.machine_name}'. Worker is heartbeating again.`,
      );
      await sb
        .from("ae_workers")
        .update({ last_stale_alert_at: null })
        .eq("id", w.id);
      recovered.push(w.machine_name);
    }
  }

  return new Response(
    JSON.stringify({ ok: true, alerted, recovered }),
    { headers: { "Content-Type": "application/json" } },
  );
});
