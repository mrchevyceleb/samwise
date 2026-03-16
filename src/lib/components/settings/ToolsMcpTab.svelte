<script lang="ts">
  import { getSettingsStore, updateSetting, type MCPServerConfig } from '$lib/stores/settings.svelte';
  import { mcpListTools, stdioMcpSpawn, stdioMcpListTools, stdioMcpStop, stdioMcpStatus } from '$lib/utils/tauri';

  const settingsStore = getSettingsStore();

  // ── Tool use state ──
  // (all from existing settings store properties)

  // ── MCP state ──
  let transportTab = $state<'stdio' | 'http'>('stdio');
  let showAddForm = $state(false);
  let newName = $state('');
  let newCommand = $state('');
  let newArgs = $state('');
  let newUrl = $state('');
  let newAuthToken = $state('');
  let newHeaders = $state('{}');
  let newTimeout = $state(20000);

  let mcpTesting = $state<Record<string, boolean>>({});
  let mcpStatus = $state<Record<string, string>>({});
  let newSlashDir = $state('');

  const PRESETS = [
    { name: 'Filesystem', command: 'npx', args: '-y @modelcontextprotocol/server-filesystem /' },
    { name: 'GitHub', command: 'npx', args: '-y @modelcontextprotocol/server-github' },
    { name: 'SQLite', command: 'npx', args: '-y @modelcontextprotocol/server-sqlite' },
    { name: 'Brave Search', command: 'npx', args: '-y @modelcontextprotocol/server-brave-search' },
    { name: 'Memory', command: 'npx', args: '-y @modelcontextprotocol/server-memory' },
  ];

  function applyPreset(preset: typeof PRESETS[number]) {
    transportTab = 'stdio';
    showAddForm = true;
    newName = preset.name;
    newCommand = preset.command;
    newArgs = preset.args;
  }

  function parseArgsString(argsStr: string): string[] {
    const result: string[] = [];
    let current = '';
    let inQuote = false;
    let quoteChar = '';
    for (const char of argsStr) {
      if (inQuote) {
        if (char === quoteChar) { inQuote = false; } else { current += char; }
      } else if (char === '"' || char === "'") {
        inQuote = true;
        quoteChar = char;
      } else if (char === ' ') {
        if (current) { result.push(current); current = ''; }
      } else {
        current += char;
      }
    }
    if (current) result.push(current);
    return result;
  }

  function addServer() {
    const id = crypto.randomUUID().slice(0, 8);
    const server: MCPServerConfig = {
      id,
      name: newName || (transportTab === 'stdio' ? newCommand : 'HTTP Server'),
      enabled: true,
      timeoutMs: newTimeout,
      transport: transportTab,
      url: transportTab === 'http' ? newUrl : '',
      authToken: transportTab === 'http' ? newAuthToken : '',
      headersJson: transportTab === 'http' ? newHeaders : '{}',
      command: transportTab === 'stdio' ? newCommand : '',
      args: transportTab === 'stdio' ? parseArgsString(newArgs) : [],
      env: {},
    };
    const current = [...settingsStore.value.mcpServers, server];
    updateSetting('mcpServers', current);
    resetForm();
  }

  function removeServer(id: string) {
    updateSetting('mcpServers', settingsStore.value.mcpServers.filter(s => s.id !== id));
  }

  function toggleServer(id: string) {
    updateSetting('mcpServers', settingsStore.value.mcpServers.map(s =>
      s.id === id ? { ...s, enabled: !s.enabled } : s
    ));
  }

  function resetForm() {
    showAddForm = false;
    newName = '';
    newCommand = '';
    newArgs = '';
    newUrl = '';
    newAuthToken = '';
    newHeaders = '{}';
    newTimeout = 20000;
  }

  async function testServer(server: MCPServerConfig) {
    mcpTesting[server.id] = true;
    mcpStatus[server.id] = 'Testing...';
    try {
      if (server.transport === 'stdio') {
        const status = await stdioMcpStatus(server.id).catch(() => 'stopped');
        if (status !== 'running') {
          await stdioMcpSpawn(server.id, server.command, server.args, server.env);
        }
        const raw = await stdioMcpListTools(server.id);
        const tools = JSON.parse(raw);
        const count = Array.isArray(tools) ? tools.length : (tools.tools?.length || 0);
        mcpStatus[server.id] = `Connected. ${count} tool${count !== 1 ? 's' : ''} available.`;
      } else {
        const raw = await mcpListTools(server.url, server.authToken || undefined, server.headersJson || undefined, server.timeoutMs);
        const tools = JSON.parse(raw);
        const count = Array.isArray(tools) ? tools.length : (tools.tools?.length || 0);
        mcpStatus[server.id] = `Connected. ${count} tool${count !== 1 ? 's' : ''} available.`;
      }
    } catch (e) {
      mcpStatus[server.id] = `Failed: ${e instanceof Error ? e.message : String(e)}`;
    } finally {
      mcpTesting[server.id] = false;
    }
  }
</script>

<div style="display: flex; flex-direction: column; gap: 20px;">
  <!-- Tool Use Controls -->
  <div style="display: flex; flex-direction: column; gap: 12px;">
    <div style="font-size: 13px; font-weight: 600; color: var(--text-primary); padding-bottom: 4px; border-bottom: 1px solid var(--border-default);">Tool Use</div>

    <div style="display: flex; align-items: center; gap: 16px; padding: 4px 0;">
      <div style="flex: 1;">
        <div style="font-size: 13px; color: var(--text-primary);">Enable Tool Use</div>
        <div style="font-size: 11px; color: var(--text-muted); margin-top: 2px;">Allow AI to read/write files, run commands</div>
      </div>
      <label style="position: relative; display: inline-block; width: 36px; height: 20px; cursor: pointer;">
        <input type="checkbox" checked={settingsStore.value.aiEnableToolUse} onchange={(e) => updateSetting('aiEnableToolUse', (e.target as HTMLInputElement).checked)} style="opacity: 0; width: 0; height: 0;" />
        <span style="position: absolute; inset: 0; background: {settingsStore.value.aiEnableToolUse ? 'var(--accent-primary)' : 'var(--border-default)'}; border-radius: 10px; transition: background 0.2s ease;">
          <span style="position: absolute; top: 2px; left: {settingsStore.value.aiEnableToolUse ? '18px' : '2px'}; width: 16px; height: 16px; background: white; border-radius: 50%; transition: left 0.2s ease;"></span>
        </span>
      </label>
    </div>

    <div style="display: flex; align-items: center; gap: 16px; padding: 4px 0;">
      <div style="flex: 1;">
        <div style="font-size: 13px; color: var(--text-primary);">Confirm Writes</div>
        <div style="font-size: 11px; color: var(--text-muted); margin-top: 2px;">Ask before file modifications</div>
      </div>
      <label style="position: relative; display: inline-block; width: 36px; height: 20px; cursor: pointer;">
        <input type="checkbox" checked={settingsStore.value.aiConfirmWrites} onchange={(e) => updateSetting('aiConfirmWrites', (e.target as HTMLInputElement).checked)} style="opacity: 0; width: 0; height: 0;" />
        <span style="position: absolute; inset: 0; background: {settingsStore.value.aiConfirmWrites ? 'var(--accent-primary)' : 'var(--border-default)'}; border-radius: 10px; transition: background 0.2s ease;">
          <span style="position: absolute; top: 2px; left: {settingsStore.value.aiConfirmWrites ? '18px' : '2px'}; width: 16px; height: 16px; background: white; border-radius: 50%; transition: left 0.2s ease;"></span>
        </span>
      </label>
    </div>

    <div style="display: flex; align-items: center; gap: 16px; padding: 4px 0;">
      <div style="flex: 1;">
        <div style="font-size: 13px; color: var(--text-primary);">YOLO Mode</div>
        <div style="font-size: 11px; color: {settingsStore.value.aiYoloMode ? 'rgb(248, 113, 113)' : 'var(--text-muted)'}; margin-top: 2px;">
          {settingsStore.value.aiYoloMode ? 'AI will modify files without confirmation!' : 'Skip all confirmations (dangerous)'}
        </div>
      </div>
      <label style="position: relative; display: inline-block; width: 36px; height: 20px; cursor: pointer;">
        <input type="checkbox" checked={settingsStore.value.aiYoloMode} onchange={(e) => updateSetting('aiYoloMode', (e.target as HTMLInputElement).checked)} style="opacity: 0; width: 0; height: 0;" />
        <span style="position: absolute; inset: 0; background: {settingsStore.value.aiYoloMode ? 'rgb(248, 113, 113)' : 'var(--border-default)'}; border-radius: 10px; transition: background 0.2s ease;">
          <span style="position: absolute; top: 2px; left: {settingsStore.value.aiYoloMode ? '18px' : '2px'}; width: 16px; height: 16px; background: white; border-radius: 50%; transition: left 0.2s ease;"></span>
        </span>
      </label>
    </div>

    <div style="display: flex; align-items: center; gap: 16px; padding: 4px 0;">
      <div style="flex: 1;">
        <div style="font-size: 13px; color: var(--text-primary);">Max Tool Iterations</div>
        <div style="font-size: 11px; color: var(--text-muted); margin-top: 2px;">Max tool-use loops per message</div>
      </div>
      <input
        type="number" min="1" max="100"
        value={settingsStore.value.aiMaxToolIterations}
        onchange={(e) => updateSetting('aiMaxToolIterations', parseInt((e.target as HTMLInputElement).value) || 75)}
        style="width: 64px; padding: 6px 10px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 6px; color: var(--text-primary); font-size: 12px; outline: none; text-align: center;"
      />
    </div>
  </div>

  <!-- MCP Servers -->
  <div style="display: flex; flex-direction: column; gap: 12px;">
    <div style="display: flex; align-items: center; gap: 8px;">
      <div style="font-size: 13px; font-weight: 600; color: var(--text-primary); padding-bottom: 4px; border-bottom: 1px solid var(--border-default); flex: 1;">MCP Servers</div>
    </div>

    <!-- Quick-add presets -->
    <div style="display: flex; flex-wrap: wrap; gap: 6px;">
      {#each PRESETS as preset}
        <button
          onclick={() => applyPreset(preset)}
          style="padding: 4px 10px; border: 1px solid var(--border-default); border-radius: 6px; cursor: pointer; font-size: 11px; font-family: var(--font-ui); background: var(--bg-primary); color: var(--text-secondary); transition: all 0.15s ease;"
          onmouseenter={(e) => { const t = e.currentTarget as HTMLElement; t.style.borderColor = 'var(--accent-dim)'; t.style.color = 'var(--accent-primary)'; }}
          onmouseleave={(e) => { const t = e.currentTarget as HTMLElement; t.style.borderColor = 'var(--border-default)'; t.style.color = 'var(--text-secondary)'; }}
        >
          + {preset.name}
        </button>
      {/each}
    </div>

    <!-- Configured servers -->
    {#each settingsStore.value.mcpServers as server (server.id)}
      <div style="padding: 10px 14px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 8px; display: flex; flex-direction: column; gap: 8px;">
        <div style="display: flex; align-items: center; gap: 10px;">
          <span style="font-size: 12px; font-weight: 600; color: var(--text-primary); flex: 1;">{server.name}</span>
          <span style="font-size: 10px; padding: 2px 6px; border-radius: 4px; background: {server.transport === 'stdio' ? 'rgba(74, 222, 128, 0.1)' : 'rgba(96, 165, 250, 0.1)'}; color: {server.transport === 'stdio' ? 'rgb(74, 222, 128)' : 'rgb(96, 165, 250)'}; font-family: var(--font-mono);">
            {server.transport}
          </span>
          <label style="position: relative; display: inline-block; width: 32px; height: 18px; cursor: pointer;">
            <input type="checkbox" checked={server.enabled} onchange={() => toggleServer(server.id)} style="opacity: 0; width: 0; height: 0;" />
            <span style="position: absolute; inset: 0; background: {server.enabled ? 'var(--accent-primary)' : 'var(--border-default)'}; border-radius: 9px; transition: background 0.2s ease;">
              <span style="position: absolute; top: 2px; left: {server.enabled ? '16px' : '2px'}; width: 14px; height: 14px; background: white; border-radius: 50%; transition: left 0.2s ease;"></span>
            </span>
          </label>
          <button
            onclick={() => testServer(server)}
            disabled={mcpTesting[server.id]}
            style="padding: 3px 8px; border: 1px solid var(--border-default); border-radius: 4px; cursor: {mcpTesting[server.id] ? 'not-allowed' : 'pointer'}; font-size: 10px; font-family: var(--font-ui); background: var(--bg-surface); color: var(--text-secondary); opacity: {mcpTesting[server.id] ? '0.5' : '1'}; transition: all 0.15s ease;"
          >
            {mcpTesting[server.id] ? 'Testing...' : 'Test'}
          </button>
          <button
            onclick={() => removeServer(server.id)}
            style="padding: 3px 8px; border: 1px solid var(--border-default); border-radius: 4px; cursor: pointer; font-size: 10px; font-family: var(--font-ui); background: var(--bg-surface); color: var(--text-muted); transition: all 0.15s ease;"
            onmouseenter={(e) => { const t = e.currentTarget as HTMLElement; t.style.color = 'rgb(248, 113, 113)'; t.style.borderColor = 'rgba(248, 113, 113, 0.3)'; }}
            onmouseleave={(e) => { const t = e.currentTarget as HTMLElement; t.style.color = 'var(--text-muted)'; t.style.borderColor = 'var(--border-default)'; }}
          >
            Remove
          </button>
        </div>
        <div style="font-size: 11px; color: var(--text-muted); font-family: var(--font-mono);">
          {#if server.transport === 'stdio'}
            {server.command} {server.args.join(' ')}
          {:else}
            {server.url}
          {/if}
        </div>
        {#if mcpStatus[server.id]}
          <div style="font-size: 11px; color: {mcpStatus[server.id]?.startsWith('Connected') ? 'rgb(74, 222, 128)' : mcpStatus[server.id]?.startsWith('Failed') ? 'rgb(248, 113, 113)' : 'var(--text-muted)'};">
            {mcpStatus[server.id]}
          </div>
        {/if}
      </div>
    {/each}

    {#if settingsStore.value.mcpServers.length === 0 && !showAddForm}
      <div style="padding: 20px; border: 1px dashed var(--border-default); border-radius: 8px; text-align: center;">
        <span style="color: var(--text-muted); font-size: 12px;">No MCP servers configured. Use a preset above or add one manually.</span>
      </div>
    {/if}

    <!-- Add Server button -->
    {#if !showAddForm}
      <button
        onclick={() => showAddForm = true}
        style="padding: 8px 16px; border: 1px dashed var(--border-default); border-radius: 8px; cursor: pointer; font-size: 12px; font-family: var(--font-ui); background: transparent; color: var(--text-secondary); transition: all 0.15s ease;"
        onmouseenter={(e) => { const t = e.currentTarget as HTMLElement; t.style.borderColor = 'var(--accent-dim)'; t.style.color = 'var(--accent-primary)'; }}
        onmouseleave={(e) => { const t = e.currentTarget as HTMLElement; t.style.borderColor = 'var(--border-default)'; t.style.color = 'var(--text-secondary)'; }}
      >
        + Add Server
      </button>
    {/if}

    <!-- Add Server Form -->
    {#if showAddForm}
      <div style="padding: 14px; background: var(--bg-primary); border: 1px solid var(--accent-dim); border-radius: 8px; display: flex; flex-direction: column; gap: 12px;">
        <div style="font-size: 13px; font-weight: 600; color: var(--accent-primary);">Add MCP Server</div>

        <!-- Transport tabs -->
        <div style="display: flex; gap: 4px;">
          <button
            onclick={() => transportTab = 'stdio'}
            style="padding: 5px 14px; border: 1px solid {transportTab === 'stdio' ? 'var(--accent-primary)' : 'var(--border-default)'}; border-radius: 6px; cursor: pointer; font-size: 11px; font-family: var(--font-mono); background: {transportTab === 'stdio' ? 'color-mix(in srgb, var(--accent-primary) 10%, transparent)' : 'transparent'}; color: {transportTab === 'stdio' ? 'var(--accent-primary)' : 'var(--text-secondary)'}; transition: all 0.15s ease;"
          >
            stdio (local)
          </button>
          <button
            onclick={() => transportTab = 'http'}
            style="padding: 5px 14px; border: 1px solid {transportTab === 'http' ? 'var(--accent-primary)' : 'var(--border-default)'}; border-radius: 6px; cursor: pointer; font-size: 11px; font-family: var(--font-mono); background: {transportTab === 'http' ? 'color-mix(in srgb, var(--accent-primary) 10%, transparent)' : 'transparent'}; color: {transportTab === 'http' ? 'var(--accent-primary)' : 'var(--text-secondary)'}; transition: all 0.15s ease;"
          >
            http (remote)
          </button>
        </div>

        <!-- Name -->
        <input
          type="text"
          bind:value={newName}
          placeholder="Server name"
          style="padding: 7px 10px; background: var(--bg-surface); border: 1px solid var(--border-default); border-radius: 6px; color: var(--text-primary); font-size: 12px; outline: none;"
        />

        {#if transportTab === 'stdio'}
          <input
            type="text"
            bind:value={newCommand}
            placeholder="Command (e.g. npx, node, python)"
            style="padding: 7px 10px; background: var(--bg-surface); border: 1px solid var(--border-default); border-radius: 6px; color: var(--text-primary); font-size: 12px; font-family: var(--font-mono); outline: none;"
          />
          <input
            type="text"
            bind:value={newArgs}
            placeholder="Arguments (space-separated)"
            style="padding: 7px 10px; background: var(--bg-surface); border: 1px solid var(--border-default); border-radius: 6px; color: var(--text-primary); font-size: 12px; font-family: var(--font-mono); outline: none;"
          />
        {:else}
          <input
            type="text"
            bind:value={newUrl}
            placeholder="Server URL (e.g. https://mcp.example.com)"
            style="padding: 7px 10px; background: var(--bg-surface); border: 1px solid var(--border-default); border-radius: 6px; color: var(--text-primary); font-size: 12px; font-family: var(--font-mono); outline: none;"
          />
          <input
            type="text"
            bind:value={newAuthToken}
            placeholder="Bearer token (optional)"
            style="padding: 7px 10px; background: var(--bg-surface); border: 1px solid var(--border-default); border-radius: 6px; color: var(--text-primary); font-size: 12px; font-family: var(--font-mono); outline: none;"
          />
        {/if}

        <!-- Actions -->
        <div style="display: flex; gap: 8px; justify-content: flex-end;">
          <button
            onclick={resetForm}
            style="padding: 6px 14px; border: 1px solid var(--border-default); border-radius: 6px; cursor: pointer; font-size: 12px; background: transparent; color: var(--text-secondary); transition: all 0.15s ease;"
          >
            Cancel
          </button>
          <button
            onclick={addServer}
            disabled={transportTab === 'stdio' ? !newCommand.trim() : !newUrl.trim()}
            style="padding: 6px 14px; border: 1px solid var(--accent-primary); border-radius: 6px; cursor: pointer; font-size: 12px; font-weight: 600; background: color-mix(in srgb, var(--accent-primary) 10%, transparent); color: var(--accent-primary); transition: all 0.15s ease; opacity: {(transportTab === 'stdio' ? !newCommand.trim() : !newUrl.trim()) ? '0.4' : '1'};"
          >
            Add Server
          </button>
        </div>
      </div>
    {/if}
  </div>

  <!-- Slash Command Directories -->
  <div style="display: flex; flex-direction: column; gap: 12px;">
    <div style="font-size: 13px; font-weight: 600; color: var(--text-primary); padding-bottom: 4px; border-bottom: 1px solid var(--border-default);">Slash Command Directories</div>
    <div style="font-size: 11px; color: var(--text-muted);">Load custom slash commands from markdown or JSON files in these directories.</div>

    {#each settingsStore.value.aiSlashCommandDirs as dir, i}
      <div style="display: flex; align-items: center; gap: 8px;">
        <span style="flex: 1; font-size: 12px; color: var(--text-primary); font-family: var(--font-mono); padding: 6px 10px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 6px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">{dir}</span>
        <button
          onclick={() => {
            const updated = settingsStore.value.aiSlashCommandDirs.filter((_, idx) => idx !== i);
            updateSetting('aiSlashCommandDirs', updated);
          }}
          style="padding: 4px 8px; border: 1px solid var(--border-default); border-radius: 4px; cursor: pointer; font-size: 10px; background: var(--bg-surface); color: var(--text-muted); transition: all 0.15s ease;"
          onmouseenter={(e) => { const t = e.currentTarget as HTMLElement; t.style.color = 'rgb(248, 113, 113)'; }}
          onmouseleave={(e) => { const t = e.currentTarget as HTMLElement; t.style.color = 'var(--text-muted)'; }}
        >
          Remove
        </button>
      </div>
    {/each}

    {#if settingsStore.value.aiSlashCommandDirs.length === 0}
      <div style="padding: 12px; border: 1px dashed var(--border-default); border-radius: 6px; text-align: center;">
        <span style="color: var(--text-muted); font-size: 11px;">No custom slash command directories configured.</span>
      </div>
    {/if}

    <!-- Add directory -->
    <div style="display: flex; gap: 8px;">
      <input
        type="text"
        bind:value={newSlashDir}
        placeholder="Path to slash command directory..."
        style="flex: 1; padding: 7px 10px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 6px; color: var(--text-primary); font-size: 12px; font-family: var(--font-mono); outline: none;"
        onkeydown={(e) => {
          if (e.key === 'Enter' && newSlashDir.trim()) {
            updateSetting('aiSlashCommandDirs', [...settingsStore.value.aiSlashCommandDirs, newSlashDir.trim()]);
            newSlashDir = '';
          }
        }}
      />
      <button
        onclick={() => {
          if (newSlashDir.trim()) {
            updateSetting('aiSlashCommandDirs', [...settingsStore.value.aiSlashCommandDirs, newSlashDir.trim()]);
            newSlashDir = '';
          }
        }}
        disabled={!newSlashDir.trim()}
        style="padding: 6px 14px; border: 1px solid var(--border-default); border-radius: 6px; cursor: {!newSlashDir.trim() ? 'not-allowed' : 'pointer'}; font-size: 12px; background: var(--bg-primary); color: var(--text-secondary); opacity: {!newSlashDir.trim() ? '0.4' : '1'}; transition: all 0.15s ease;"
      >
        Add
      </button>
    </div>
  </div>
</div>
