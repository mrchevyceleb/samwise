<script lang="ts">
  let projectName = $state('');
  let selectedConfig = $state('dev');
  let testStatus = $state<'idle' | 'testing' | 'success' | 'error'>('idle');
  let testBtnHovered = $state(false);

  const configs = ['dev', 'staging', 'production'];

  async function testConnection() {
    testStatus = 'testing';
    // Simulate test (real implementation would call Doppler API)
    setTimeout(() => {
      testStatus = projectName.trim() ? 'success' : 'error';
      setTimeout(() => { testStatus = 'idle'; }, 3000);
    }, 1500);
  }
</script>

<div style="display: flex; flex-direction: column; gap: 20px;">
  <div style="padding: 12px 16px; background: rgba(255, 214, 10, 0.06); border: 1px solid rgba(255, 214, 10, 0.15); border-radius: 8px;">
    <div style="font-size: 13px; color: var(--banana-yellow); font-weight: 600; margin-bottom: 4px;">Doppler Secrets Management</div>
    <div style="font-size: 12px; color: var(--text-secondary);">
      Doppler manages your environment variables and secrets securely. Configure your project to pull secrets at runtime instead of storing them in .env files.
    </div>
  </div>

  <!-- Project Name -->
  <div style="display: flex; flex-direction: column; gap: 8px;">
    <span style="font-size: 13px; font-weight: 600; color: var(--text-primary);">Project Name</span>
    <input
      type="text"
      bind:value={projectName}
      placeholder="e.g. banana-code"
      style="padding: 8px 12px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 6px; color: var(--text-primary); font-size: 12px; font-family: var(--font-mono); outline: none;"
    />
  </div>

  <!-- Config Selector -->
  <div style="display: flex; flex-direction: column; gap: 8px;">
    <span style="font-size: 13px; font-weight: 600; color: var(--text-primary);">Config Environment</span>
    <div style="display: flex; gap: 8px;">
      {#each configs as config}
        <button
          onclick={() => selectedConfig = config}
          style="padding: 6px 16px; border: 1px solid {selectedConfig === config ? 'var(--banana-yellow)' : 'var(--border-default)'}; border-radius: 6px; cursor: pointer; font-size: 12px; font-family: var(--font-ui); transition: all 0.15s ease; text-transform: capitalize; background: {selectedConfig === config ? 'rgba(255, 214, 10, 0.1)' : 'var(--bg-primary)'}; color: {selectedConfig === config ? 'var(--banana-yellow)' : 'var(--text-secondary)'};"
          onmouseenter={(e) => { if (selectedConfig !== config) { const t = e.currentTarget as HTMLElement; t.style.borderColor = 'var(--banana-yellow-dim)'; }}}
          onmouseleave={(e) => { if (selectedConfig !== config) { const t = e.currentTarget as HTMLElement; t.style.borderColor = 'var(--border-default)'; }}}
        >
          {config}
        </button>
      {/each}
    </div>
  </div>

  <!-- Test Connection -->
  <div style="display: flex; align-items: center; gap: 12px;">
    <button
      onclick={testConnection}
      onmouseenter={() => testBtnHovered = true}
      onmouseleave={() => testBtnHovered = false}
      disabled={testStatus === 'testing'}
      style="padding: 8px 20px; border: none; border-radius: 6px; cursor: pointer; font-size: 12px; font-weight: 600; font-family: var(--font-ui); transition: all 0.15s ease; background: {testBtnHovered ? 'var(--banana-yellow-hover)' : 'var(--banana-yellow)'}; color: #0D1117; transform: {testBtnHovered ? 'translateY(-1px)' : 'translateY(0)'}; box-shadow: {testBtnHovered ? '0 4px 12px rgba(255, 214, 10, 0.3)' : 'none'};"
    >
      {testStatus === 'testing' ? 'Testing...' : 'Test Connection'}
    </button>

    {#if testStatus === 'success'}
      <div style="display: flex; align-items: center; gap: 6px;">
        <span style="width: 8px; height: 8px; border-radius: 50%; background: var(--accent-green); display: inline-block;"></span>
        <span style="font-size: 12px; color: var(--accent-green);">Connected</span>
      </div>
    {:else if testStatus === 'error'}
      <div style="display: flex; align-items: center; gap: 6px;">
        <span style="width: 8px; height: 8px; border-radius: 50%; background: var(--accent-red); display: inline-block;"></span>
        <span style="font-size: 12px; color: var(--accent-red);">Connection failed</span>
      </div>
    {/if}
  </div>
</div>
