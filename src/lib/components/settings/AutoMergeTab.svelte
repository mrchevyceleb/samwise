<script lang="ts">
  import { getSettingsStore, updateSetting } from '$lib/stores/settings.svelte';

  const settingsStore = getSettingsStore();

  let hovered = $state<string | null>(null);

  let enabled = $derived(settingsStore.value.autoMergeEnabled);
  let minScore = $derived(settingsStore.value.autoMergeMinScore);
  let maxDiff = $derived(settingsStore.value.autoMergeMaxDiffLines);
  let prReviewEnabled = $derived(settingsStore.value.autoPrReviewEnabled);
  let autoFixEnabled = $derived(settingsStore.value.autoFixFromFixesNeededEnabled);
  let visualQaEnabled = $derived(settingsStore.value.visualQaEnabled);

  function toggleEnabled() {
    updateSetting('autoMergeEnabled', !enabled);
  }

  function togglePrReviewEnabled() {
    updateSetting('autoPrReviewEnabled', !prReviewEnabled);
  }

  function toggleAutoFix() {
    updateSetting('autoFixFromFixesNeededEnabled', !autoFixEnabled);
  }

  function toggleVisualQa() {
    updateSetting('visualQaEnabled', !visualQaEnabled);
  }

  function setMinScore(v: number) {
    if (Number.isFinite(v)) {
      const clamped = Math.max(1, Math.min(10, Math.round(v)));
      updateSetting('autoMergeMinScore', clamped);
    }
  }

  function setMaxDiff(v: number) {
    if (Number.isFinite(v)) {
      const clamped = Math.max(1, Math.min(5000, Math.round(v)));
      updateSetting('autoMergeMaxDiffLines', clamped);
    }
  }
</script>

<div style="display: flex; flex-direction: column; gap: 20px;">
  <div style="font-size: 13px; font-weight: 600; color: var(--text-primary); padding-bottom: 4px; border-bottom: 1px solid var(--border-default);">
    Auto-Merge Gate
  </div>

  <div style="font-size: 12px; color: var(--text-secondary); line-height: 1.5;">
    After Sam opens a PR, run a Codex review (gpt-5.5, xhigh reasoning) and auto-merge only when every gate passes.
    Gates: feature toggle on, no blocker paths touched (migrations, worker.rs, chat.rs, auth/secret/token files, dep manifests),
    diff under the line cap, the lowest review dimension at or above the minimum score, no review-flagged blockers, and CI green.
    If any gate fails, the PR stays in review with a comment explaining why.
  </div>

  <!-- Master toggle -->
  <div
    style="display: flex; align-items: center; justify-content: space-between; padding: 14px 16px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 10px;"
    onmouseenter={() => hovered = 'master'}
    onmouseleave={() => hovered = null}
  >
    <div>
      <div style="font-size: 13px; font-weight: 600; color: var(--text-primary);">Auto-merge PRs when review passes</div>
      <div style="font-size: 11px; color: var(--text-muted); margin-top: 2px;">Off by default. Turn on once you trust the reviewer.</div>
    </div>
    <button
      onclick={toggleEnabled}
      role="switch"
      aria-checked={enabled}
      aria-label="Toggle auto-merge"
      style="
        position: relative; width: 44px; height: 24px; border-radius: 12px; cursor: pointer;
        background: {enabled ? '#6366f1' : 'var(--bg-elevated)'};
        border: 1px solid {enabled ? '#6366f1' : 'var(--border-default)'};
        transition: all 0.2s ease;
        transform: {hovered === 'master' ? 'scale(1.05)' : 'scale(1)'};
      "
    >
      <span style="
        position: absolute; top: 2px; left: {enabled ? '22px' : '2px'};
        width: 18px; height: 18px; border-radius: 50%;
        background: {enabled ? 'white' : 'var(--text-muted)'};
        transition: all 0.2s ease;
        box-shadow: 0 1px 3px rgba(0,0,0,0.3);
      "></span>
    </button>
  </div>

  <!-- Numeric thresholds -->
  <div style="display: flex; flex-direction: column; gap: 12px; opacity: {enabled ? 1 : 0.5}; transition: opacity 0.2s ease;">
    <div style="display: flex; align-items: center; justify-content: space-between; padding: 12px 16px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 8px;">
      <div>
        <div style="font-size: 13px; font-weight: 500; color: var(--text-primary);">Minimum score (1 to 10)</div>
        <div style="font-size: 11px; color: var(--text-muted); margin-top: 2px;">Lowest dimension across correctness, blast radius, test coverage, reversibility, and intent match. Default 8.</div>
      </div>
      <input
        type="number"
        min="1"
        max="10"
        step="1"
        placeholder="8"
        value={minScore}
        disabled={!enabled}
        oninput={(e) => setMinScore(parseInt((e.currentTarget as HTMLInputElement).value, 10))}
        style="width: 72px; padding: 6px 8px; background: var(--bg-elevated); border: 1px solid var(--border-default); border-radius: 6px; color: var(--text-primary); font-size: 13px; text-align: right;"
      />
    </div>

    <div style="display: flex; align-items: center; justify-content: space-between; padding: 12px 16px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 8px;">
      <div>
        <div style="font-size: 13px; font-weight: 500; color: var(--text-primary);">Max diff lines</div>
        <div style="font-size: 11px; color: var(--text-muted); margin-top: 2px;">Combined additions plus deletions. Larger PRs always go to review. Default 400, max 5000.</div>
      </div>
      <input
        type="number"
        min="1"
        max="5000"
        step="50"
        placeholder="400"
        value={maxDiff}
        disabled={!enabled}
        oninput={(e) => setMaxDiff(parseInt((e.currentTarget as HTMLInputElement).value, 10))}
        style="width: 96px; padding: 6px 8px; background: var(--bg-elevated); border: 1px solid var(--border-default); border-radius: 6px; color: var(--text-primary); font-size: 13px; text-align: right;"
      />
    </div>
  </div>

  <!-- Codex $samwise-pr-review pass (runs only when auto-merge is OFF) -->
  <div style="font-size: 13px; font-weight: 600; color: var(--text-primary); padding-bottom: 4px; border-top: 1px solid var(--border-default); padding-top: 20px; margin-top: 4px;">
    Codex PR Review (when auto-merge is off)
  </div>

  <div style="font-size: 12px; color: var(--text-secondary); line-height: 1.5;">
    When auto-merge is off, Sam runs <code>$samwise-pr-review</code> via the Codex CLI on every new PR
    and moves the card to <strong>Ready to Merge</strong> or <strong>Fixes Needed</strong> based on the verdict.
    Cards you drag back from Fixes Needed to Review get re-reviewed automatically.
    If Codex can't produce a verdict (rate limit, logged out, timeout), the card stays in Review
    and Sam posts the raw output as a comment.
  </div>

  <div
    style="display: flex; align-items: center; justify-content: space-between; padding: 14px 16px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 10px; opacity: {enabled ? 0.5 : 1}; transition: opacity 0.2s ease;"
    onmouseenter={() => hovered = 'prreview'}
    onmouseleave={() => hovered = null}
  >
    <div>
      <div style="font-size: 13px; font-weight: 600; color: var(--text-primary);">Auto-run $samwise-pr-review on new PRs</div>
      <div style="font-size: 11px; color: var(--text-muted); margin-top: 2px;">Disabled while auto-merge is on (auto-merge already runs its own Codex review).</div>
    </div>
    <button
      onclick={togglePrReviewEnabled}
      disabled={enabled}
      role="switch"
      aria-checked={prReviewEnabled}
      aria-label="Toggle Codex PR review"
      style="
        position: relative; width: 44px; height: 24px; border-radius: 12px; cursor: {enabled ? 'not-allowed' : 'pointer'};
        background: {prReviewEnabled ? '#6366f1' : 'var(--bg-elevated)'};
        border: 1px solid {prReviewEnabled ? '#6366f1' : 'var(--border-default)'};
        transition: all 0.2s ease;
        transform: {hovered === 'prreview' && !enabled ? 'scale(1.05)' : 'scale(1)'};
      "
    >
      <span style="
        position: absolute; top: 2px; left: {prReviewEnabled ? '22px' : '2px'};
        width: 18px; height: 18px; border-radius: 50%;
        background: {prReviewEnabled ? 'white' : 'var(--text-muted)'};
        transition: all 0.2s ease;
        box-shadow: 0 1px 3px rgba(0,0,0,0.3);
      "></span>
    </button>
  </div>

  <!-- Auto-fix loop -->
  <div
    style="display: flex; align-items: center; justify-content: space-between; padding: 14px 16px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 10px; opacity: {enabled || !prReviewEnabled ? 0.5 : 1}; transition: opacity 0.2s ease;"
    onmouseenter={() => hovered = 'autofix'}
    onmouseleave={() => hovered = null}
  >
    <div>
      <div style="font-size: 13px; font-weight: 600; color: var(--text-primary);">Auto-fix cards in Fixes Needed</div>
      <div style="font-size: 11px; color: var(--text-muted); margin-top: 2px;">When Codex returns fix_issues, Sam runs Claude Code on the same worktree, addresses the blockers, and pushes. Max 3 cycles per card. Skipped when Codex flags the blockers as needing human judgment.</div>
    </div>
    <button
      onclick={toggleAutoFix}
      disabled={enabled || !prReviewEnabled}
      role="switch"
      aria-checked={autoFixEnabled}
      aria-label="Toggle auto-fix on fixes needed"
      style="
        position: relative; width: 44px; height: 24px; border-radius: 12px; cursor: {enabled || !prReviewEnabled ? 'not-allowed' : 'pointer'};
        background: {autoFixEnabled ? '#6366f1' : 'var(--bg-elevated)'};
        border: 1px solid {autoFixEnabled ? '#6366f1' : 'var(--border-default)'};
        transition: all 0.2s ease;
        transform: {hovered === 'autofix' && !enabled && prReviewEnabled ? 'scale(1.05)' : 'scale(1)'};
      "
    >
      <span style="
        position: absolute; top: 2px; left: {autoFixEnabled ? '22px' : '2px'};
        width: 18px; height: 18px; border-radius: 50%;
        background: {autoFixEnabled ? 'white' : 'var(--text-muted)'};
        transition: all 0.2s ease;
        box-shadow: 0 1px 3px rgba(0,0,0,0.3);
      "></span>
    </button>
  </div>

  <!-- Visual QA -->
  <div style="font-size: 13px; font-weight: 600; color: var(--text-primary); padding-bottom: 4px; border-top: 1px solid var(--border-default); padding-top: 20px; margin-top: 4px;">
    Visual QA
  </div>

  <div style="font-size: 12px; color: var(--text-secondary); line-height: 1.5;">
    When on, Sam starts a dev server, takes before/after screenshots (desktop and mobile), and runs a Claude vision pass that flags regressions
    and feeds the explanation back to Claude Code as a fix-it prompt (up to 3 attempts). Off by default. Currently the screenshotter has no
    authentication support, so for auth-walled apps it only sees the login page and rubber-stamps every PR. Leave off until per-project storage
    state is wired up.
  </div>

  <div
    style="display: flex; align-items: center; justify-content: space-between; padding: 14px 16px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 10px;"
    onmouseenter={() => hovered = 'visualqa'}
    onmouseleave={() => hovered = null}
  >
    <div>
      <div style="font-size: 13px; font-weight: 600; color: var(--text-primary);">Run Visual QA on every code task</div>
      <div style="font-size: 11px; color: var(--text-muted); margin-top: 2px;">Off by default. Turn on once your project has authenticated screenshots set up.</div>
    </div>
    <button
      onclick={toggleVisualQa}
      role="switch"
      aria-checked={visualQaEnabled}
      aria-label="Toggle Visual QA"
      style="
        position: relative; width: 44px; height: 24px; border-radius: 12px; cursor: pointer;
        background: {visualQaEnabled ? '#6366f1' : 'var(--bg-elevated)'};
        border: 1px solid {visualQaEnabled ? '#6366f1' : 'var(--border-default)'};
        transition: all 0.2s ease;
        transform: {hovered === 'visualqa' ? 'scale(1.05)' : 'scale(1)'};
      "
    >
      <span style="
        position: absolute; top: 2px; left: {visualQaEnabled ? '22px' : '2px'};
        width: 18px; height: 18px; border-radius: 50%;
        background: {visualQaEnabled ? 'white' : 'var(--text-muted)'};
        transition: all 0.2s ease;
        box-shadow: 0 1px 3px rgba(0,0,0,0.3);
      "></span>
    </button>
  </div>
</div>
