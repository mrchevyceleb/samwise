<script lang="ts">
  import ModelsTab from './ModelsTab.svelte';
  import GenerationTab from './GenerationTab.svelte';
  import ToolsMcpTab from './ToolsMcpTab.svelte';

  type Tab = 'models' | 'generation' | 'tools';
  let activeTab = $state<Tab>('models');
  let hoveredTab = $state<string | null>(null);

  const tabs: { id: Tab; label: string }[] = [
    { id: 'models', label: 'Models' },
    { id: 'generation', label: 'Generation' },
    { id: 'tools', label: 'Tools & MCP' },
  ];
</script>

<div style="display: flex; flex-direction: column; gap: 16px;">
  <!-- Tab bar -->
  <div style="display: flex; gap: 4px; border-bottom: 1px solid var(--border-default); padding-bottom: 0;">
    {#each tabs as tab (tab.id)}
      <button
        onclick={() => activeTab = tab.id}
        onmouseenter={() => hoveredTab = tab.id}
        onmouseleave={() => hoveredTab = null}
        style="padding: 8px 16px; border: none; cursor: pointer; font-size: 12px; font-family: var(--font-ui); font-weight: {activeTab === tab.id ? '600' : '400'}; transition: all 0.12s ease; background: transparent; color: {activeTab === tab.id ? 'var(--banana-yellow)' : hoveredTab === tab.id ? 'var(--text-primary)' : 'var(--text-secondary)'}; border-bottom: 2px solid {activeTab === tab.id ? 'var(--banana-yellow)' : 'transparent'}; margin-bottom: -1px;"
      >
        {tab.label}
      </button>
    {/each}
  </div>

  <!-- Tab content -->
  {#if activeTab === 'models'}
    <ModelsTab />
  {:else if activeTab === 'generation'}
    <GenerationTab />
  {:else if activeTab === 'tools'}
    <ToolsMcpTab />
  {/if}
</div>
