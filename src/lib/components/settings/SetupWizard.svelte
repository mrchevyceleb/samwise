<script lang="ts">
  import { safeInvoke } from '$lib/utils/tauri';

  interface Props {
    onComplete: (machineName: string) => void;
  }
  let { onComplete }: Props = $props();

  type StepStatus = 'pending' | 'checking' | 'success' | 'error' | 'skipped';

  let currentStep = $state(0);
  let dopplerStatus = $state<StepStatus>('pending');
  let dopplerMessage = $state('');
  let supabaseStatus = $state<StepStatus>('pending');
  let supabaseMessage = $state('');
  let claudeStatus = $state<StepStatus>('pending');
  let claudeMessage = $state('');
  let ghStatus = $state<StepStatus>('pending');
  let ghMessage = $state('');
  let machineName = $state('');

  const steps = [
    { label: 'Doppler', icon: 'K' },
    { label: 'Supabase', icon: 'S' },
    { label: 'Claude Code', icon: 'C' },
    { label: 'GitHub CLI', icon: 'G' },
    { label: 'Machine', icon: 'M' },
  ];

  // Step 1: Check and load Doppler
  async function checkDoppler() {
    dopplerStatus = 'checking';
    dopplerMessage = 'Checking Doppler CLI...';

    const version = await safeInvoke<string>('check_doppler');
    if (!version) {
      dopplerStatus = 'error';
      dopplerMessage = 'Doppler CLI not found. Install it from doppler.com/docs/cli';
      return;
    }

    dopplerMessage = `Found: ${version}. Loading secrets...`;
    const config = await safeInvoke<{ url: string; anon_key: string; service_role_key: string | null }>('supabase_load_doppler');
    if (config && config.url) {
      dopplerStatus = 'success';
      dopplerMessage = `Loaded. URL: ${config.url.substring(0, 40)}...`;
    } else {
      dopplerStatus = 'error';
      dopplerMessage = 'Doppler found but failed to load agent-one secrets. Check project config.';
    }
  }

  // Step 2: Test Supabase connection
  async function checkSupabase() {
    supabaseStatus = 'checking';
    supabaseMessage = 'Testing Supabase connection...';

    const result = await safeInvoke<string>('supabase_test_connection');
    if (result) {
      supabaseStatus = 'success';
      supabaseMessage = result;
    } else {
      supabaseStatus = 'error';
      supabaseMessage = 'Connection failed. Go back and check Doppler config.';
    }
  }

  // Step 3: Check Claude Code CLI
  async function checkClaude() {
    claudeStatus = 'checking';
    claudeMessage = 'Checking Claude Code CLI...';

    const version = await safeInvoke<string>('check_claude_code');
    if (version) {
      claudeStatus = 'success';
      claudeMessage = `Found: ${version}`;
    } else {
      claudeStatus = 'error';
      claudeMessage = 'Claude Code CLI not found. Install via npm: npm install -g @anthropic-ai/claude-code';
    }
  }

  // Step 4: Check gh CLI
  async function checkGh() {
    ghStatus = 'checking';
    ghMessage = 'Checking GitHub CLI...';

    const result = await safeInvoke<string>('check_gh_auth');
    if (result) {
      ghStatus = 'success';
      ghMessage = 'Authenticated';
    } else {
      ghStatus = 'error';
      ghMessage = 'gh CLI not found or not authenticated. Run: gh auth login';
    }
  }

  async function goToStep(step: number) {
    currentStep = step;
    if (step === 0) await checkDoppler();
    if (step === 1) await checkSupabase();
    if (step === 2) await checkClaude();
    if (step === 3) await checkGh();
  }

  function canProceed(step: number): boolean {
    if (step === 0) return dopplerStatus === 'success' || dopplerStatus === 'error';
    if (step === 1) return supabaseStatus === 'success' || supabaseStatus === 'error';
    if (step === 2) return claudeStatus === 'success' || claudeStatus === 'error';
    if (step === 3) return ghStatus === 'success' || ghStatus === 'error';
    return true;
  }

  function statusColor(s: StepStatus): string {
    if (s === 'success') return '#3fb950';
    if (s === 'error') return '#f85149';
    if (s === 'checking') return '#6366f1';
    return '#6e7681';
  }

  async function finish() {
    onComplete(machineName.trim() || 'agent-one');
  }

  // Auto-start first check
  $effect(() => {
    if (currentStep === 0 && dopplerStatus === 'pending') {
      checkDoppler();
    }
  });
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div style="position: fixed; inset: 0; z-index: 2000; background: rgba(0,0,0,0.75); backdrop-filter: blur(8px); display: flex; align-items: center; justify-content: center;">
  <div style="width: 560px; background: var(--bg-surface, #161b22); border: 1px solid var(--border-default, #30363d); border-radius: 16px; box-shadow: 0 32px 80px rgba(0,0,0,0.6); overflow: hidden;">

    <!-- Header -->
    <div style="padding: 24px 28px 16px; text-align: center;">
      <div style="width: 56px; height: 56px; margin: 0 auto 12px; background: linear-gradient(135deg, #6366f1, #8b5cf6); border-radius: 14px; display: flex; align-items: center; justify-content: center; animation: bob 3s ease-in-out infinite;">
        <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.5">
          <circle cx="12" cy="8" r="5"/><path d="M3 21v-2a7 7 0 0 1 7-7h4a7 7 0 0 1 7 7v2"/><circle cx="9" cy="7" r="1" fill="white"/><circle cx="15" cy="7" r="1" fill="white"/>
        </svg>
      </div>
      <div style="font-size: 20px; font-weight: 700; color: var(--text-primary, #e6edf3);">Welcome to SamWise</div>
      <div style="font-size: 13px; color: var(--text-secondary, #8b949e); margin-top: 4px;">Let's make sure everything is wired up.</div>
    </div>

    <!-- Step indicator -->
    <div style="display: flex; justify-content: center; gap: 8px; padding: 0 28px 20px;">
      {#each steps as step, i}
        <button
          onclick={() => { if (i <= currentStep) goToStep(i); }}
          style="width: 32px; height: 32px; border-radius: 8px; border: 1px solid {i === currentStep ? '#6366f1' : 'var(--border-default, #30363d)'}; background: {i === currentStep ? 'rgba(99,102,241,0.15)' : i < currentStep ? 'rgba(63,185,80,0.1)' : 'transparent'}; color: {i === currentStep ? '#6366f1' : i < currentStep ? '#3fb950' : 'var(--text-muted, #6e7681)'}; font-size: 12px; font-weight: 700; cursor: {i <= currentStep ? 'pointer' : 'default'}; display: flex; align-items: center; justify-content: center; transition: all 0.2s;"
        >
          {#if i < currentStep}
            ok
          {:else}
            {step.icon}
          {/if}
        </button>
      {/each}
    </div>

    <!-- Step content -->
    <div style="padding: 0 28px 24px; min-height: 140px;">
      {#if currentStep === 0}
        <div style="display: flex; flex-direction: column; gap: 12px;">
          <div style="font-size: 14px; font-weight: 600; color: var(--text-primary, #e6edf3);">Step 1: Doppler Secrets</div>
          <div style="font-size: 12px; color: var(--text-secondary, #8b949e); line-height: 1.5;">SamWise uses Doppler to load Supabase credentials securely. The Doppler CLI must be installed and authenticated.</div>
          <div style="display: flex; align-items: center; gap: 8px; padding: 10px 14px; background: var(--bg-primary, #0d1117); border: 1px solid var(--border-default, #30363d); border-radius: 8px;">
            <span style="width: 8px; height: 8px; border-radius: 50%; background: {statusColor(dopplerStatus)}; {dopplerStatus === 'checking' ? 'animation: pulse-dot 1s ease-in-out infinite;' : ''}"></span>
            <span style="font-size: 12px; color: var(--text-secondary, #8b949e); flex: 1;">{dopplerMessage || 'Waiting...'}</span>
          </div>
          {#if dopplerStatus === 'error'}
            <button onclick={checkDoppler} style="align-self: flex-start; padding: 6px 14px; background: rgba(99,102,241,0.1); border: 1px solid rgba(99,102,241,0.3); border-radius: 6px; color: #6366f1; font-size: 12px; font-weight: 600; cursor: pointer;">Retry</button>
          {/if}
        </div>

      {:else if currentStep === 1}
        <div style="display: flex; flex-direction: column; gap: 12px;">
          <div style="font-size: 14px; font-weight: 600; color: var(--text-primary, #e6edf3);">Step 2: Supabase Connection</div>
          <div style="font-size: 12px; color: var(--text-secondary, #8b949e); line-height: 1.5;">Verifying we can reach Supabase with the credentials from Doppler.</div>
          <div style="display: flex; align-items: center; gap: 8px; padding: 10px 14px; background: var(--bg-primary, #0d1117); border: 1px solid var(--border-default, #30363d); border-radius: 8px;">
            <span style="width: 8px; height: 8px; border-radius: 50%; background: {statusColor(supabaseStatus)}; {supabaseStatus === 'checking' ? 'animation: pulse-dot 1s ease-in-out infinite;' : ''}"></span>
            <span style="font-size: 12px; color: var(--text-secondary, #8b949e); flex: 1;">{supabaseMessage || 'Waiting...'}</span>
          </div>
          {#if supabaseStatus === 'error'}
            <button onclick={checkSupabase} style="align-self: flex-start; padding: 6px 14px; background: rgba(99,102,241,0.1); border: 1px solid rgba(99,102,241,0.3); border-radius: 6px; color: #6366f1; font-size: 12px; font-weight: 600; cursor: pointer;">Retry</button>
          {/if}
        </div>

      {:else if currentStep === 2}
        <div style="display: flex; flex-direction: column; gap: 12px;">
          <div style="font-size: 14px; font-weight: 600; color: var(--text-primary, #e6edf3);">Step 3: Claude Code CLI</div>
          <div style="font-size: 12px; color: var(--text-secondary, #8b949e); line-height: 1.5;">The worker uses Claude Code CLI to execute tasks. It must be installed and authenticated.</div>
          <div style="display: flex; align-items: center; gap: 8px; padding: 10px 14px; background: var(--bg-primary, #0d1117); border: 1px solid var(--border-default, #30363d); border-radius: 8px;">
            <span style="width: 8px; height: 8px; border-radius: 50%; background: {statusColor(claudeStatus)}; {claudeStatus === 'checking' ? 'animation: pulse-dot 1s ease-in-out infinite;' : ''}"></span>
            <span style="font-size: 12px; color: var(--text-secondary, #8b949e); flex: 1;">{claudeMessage || 'Waiting...'}</span>
          </div>
          {#if claudeStatus === 'error'}
            <button onclick={checkClaude} style="align-self: flex-start; padding: 6px 14px; background: rgba(99,102,241,0.1); border: 1px solid rgba(99,102,241,0.3); border-radius: 6px; color: #6366f1; font-size: 12px; font-weight: 600; cursor: pointer;">Retry</button>
          {/if}
        </div>

      {:else if currentStep === 3}
        <div style="display: flex; flex-direction: column; gap: 12px;">
          <div style="font-size: 14px; font-weight: 600; color: var(--text-primary, #e6edf3);">Step 4: GitHub CLI</div>
          <div style="font-size: 12px; color: var(--text-secondary, #8b949e); line-height: 1.5;">The worker uses `gh` to create pull requests. It needs to be installed and authenticated.</div>
          <div style="display: flex; align-items: center; gap: 8px; padding: 10px 14px; background: var(--bg-primary, #0d1117); border: 1px solid var(--border-default, #30363d); border-radius: 8px;">
            <span style="width: 8px; height: 8px; border-radius: 50%; background: {statusColor(ghStatus)}; {ghStatus === 'checking' ? 'animation: pulse-dot 1s ease-in-out infinite;' : ''}"></span>
            <span style="font-size: 12px; color: var(--text-secondary, #8b949e); flex: 1;">{ghMessage || 'Waiting...'}</span>
          </div>
          {#if ghStatus === 'error'}
            <button onclick={checkGh} style="align-self: flex-start; padding: 6px 14px; background: rgba(99,102,241,0.1); border: 1px solid rgba(99,102,241,0.3); border-radius: 6px; color: #6366f1; font-size: 12px; font-weight: 600; cursor: pointer;">Retry</button>
          {/if}
        </div>

      {:else if currentStep === 4}
        <div style="display: flex; flex-direction: column; gap: 12px;">
          <div style="font-size: 14px; font-weight: 600; color: var(--text-primary, #e6edf3);">Step 5: Machine Name</div>
          <div style="font-size: 12px; color: var(--text-secondary, #8b949e); line-height: 1.5;">Give this machine a name. It shows up in the worker heartbeat so you know which machine is running tasks.</div>
          <input
            bind:value={machineName}
            placeholder="e.g. desktop-4090, laptop, trenzalore"
            style="padding: 10px 14px; background: var(--bg-primary, #0d1117); border: 1px solid var(--border-default, #30363d); border-radius: 8px; color: var(--text-primary, #e6edf3); font-size: 13px; font-family: var(--font-mono, monospace); outline: none;"
          />
        </div>
      {/if}
    </div>

    <!-- Navigation -->
    <div style="display: flex; justify-content: space-between; padding: 16px 28px; border-top: 1px solid var(--border-default, #30363d); background: var(--bg-primary, #0d1117);">
      <button
        onclick={() => { if (currentStep > 0) goToStep(currentStep - 1); }}
        disabled={currentStep === 0}
        style="padding: 8px 16px; background: transparent; border: 1px solid var(--border-default, #30363d); border-radius: 6px; color: var(--text-secondary, #8b949e); font-size: 13px; font-weight: 600; cursor: {currentStep === 0 ? 'default' : 'pointer'}; opacity: {currentStep === 0 ? 0.4 : 1};"
      >
        Back
      </button>

      {#if currentStep < 4}
        <button
          onclick={() => goToStep(currentStep + 1)}
          disabled={!canProceed(currentStep)}
          style="padding: 8px 20px; background: {canProceed(currentStep) ? '#6366f1' : 'var(--border-default, #30363d)'}; border: none; border-radius: 6px; color: white; font-size: 13px; font-weight: 600; cursor: {canProceed(currentStep) ? 'pointer' : 'default'}; opacity: {canProceed(currentStep) ? 1 : 0.5}; transition: all 0.15s;"
        >
          Next
        </button>
      {:else}
        <button
          onclick={finish}
          style="padding: 8px 20px; background: #3fb950; border: none; border-radius: 6px; color: white; font-size: 13px; font-weight: 700; cursor: pointer; transition: all 0.15s;"
        >
          Start SamWise
        </button>
      {/if}
    </div>
  </div>
</div>

<style>
  @keyframes bob {
    0%, 100% { transform: translateY(0); }
    50% { transform: translateY(-4px); }
  }
  @keyframes pulse-dot {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.4; }
  }
</style>
