<script lang="ts">
  import { getSettingsStore, updateSetting } from '$lib/stores/settings.svelte';

  const settingsStore = getSettingsStore();

  let newRule = $state('');
  let editingIndex = $state<number | null>(null);
  let editingText = $state('');
  let hoveredIndex = $state<number | null>(null);
  let hoveredBtn = $state<string | null>(null);

  function addRule() {
    const text = newRule.trim();
    if (!text) return;
    const rules = [...settingsStore.value.workerRules, text];
    updateSetting('workerRules', rules);
    newRule = '';
  }

  function removeRule(index: number) {
    const rules = settingsStore.value.workerRules.filter((_, i) => i !== index);
    updateSetting('workerRules', rules);
    if (editingIndex === index) editingIndex = null;
  }

  function startEdit(index: number) {
    editingIndex = index;
    editingText = settingsStore.value.workerRules[index];
  }

  function saveEdit(index: number) {
    const text = editingText.trim();
    if (!text) return;
    const rules = [...settingsStore.value.workerRules];
    rules[index] = text;
    updateSetting('workerRules', rules);
    editingIndex = null;
  }

  function cancelEdit() {
    editingIndex = null;
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      addRule();
    }
  }

  function handleEditKeyDown(e: KeyboardEvent, index: number) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      saveEdit(index);
    } else if (e.key === 'Escape') {
      cancelEdit();
    }
  }

  function moveRule(index: number, direction: -1 | 1) {
    const rules = [...settingsStore.value.workerRules];
    const target = index + direction;
    if (target < 0 || target >= rules.length) return;
    [rules[index], rules[target]] = [rules[target], rules[index]];
    updateSetting('workerRules', rules);
  }
</script>

<div style="display: flex; flex-direction: column; gap: 20px;">
  <div style="font-size: 13px; font-weight: 600; color: var(--text-primary); padding-bottom: 4px; border-bottom: 1px solid var(--border-default);">
    Worker Rules
  </div>

  <div style="font-size: 12px; color: var(--text-secondary); line-height: 1.5;">
    Natural language instructions the worker MUST follow before every task. These get injected into the prompt alongside CLAUDE.md.
    Use these for project-specific constraints, coding standards, or behavioral guardrails.
  </div>

  <!-- Add new rule -->
  <div style="display: flex; gap: 8px;">
    <textarea
      bind:value={newRule}
      onkeydown={handleKeyDown}
      placeholder="e.g. Always run tests before committing. Never modify migration files directly."
      rows="2"
      style="flex: 1; padding: 8px 12px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 6px; color: var(--text-primary); font-size: 13px; font-family: var(--font-ui); outline: none; resize: vertical; min-height: 40px;"
    ></textarea>
    <button
      onclick={addRule}
      onmouseenter={() => hoveredBtn = 'add'}
      onmouseleave={() => hoveredBtn = null}
      style="padding: 8px 16px; background: {hoveredBtn === 'add' ? '#818cf8' : '#6366f1'}; color: white; border: none; border-radius: 6px; font-size: 13px; font-weight: 600; cursor: pointer; transition: all 0.15s ease; align-self: flex-end; white-space: nowrap;"
    >
      Add Rule
    </button>
  </div>

  <!-- Rules list -->
  {#if settingsStore.value.workerRules.length === 0}
    <div style="padding: 24px; text-align: center; color: var(--text-muted); font-size: 13px; border: 1px dashed var(--border-default); border-radius: 8px;">
      No rules yet. Add rules above to guide the worker's behavior.
    </div>
  {:else}
    <div style="display: flex; flex-direction: column; gap: 6px;">
      {#each settingsStore.value.workerRules as rule, index (index)}
        <div
          onmouseenter={() => hoveredIndex = index}
          onmouseleave={() => hoveredIndex = null}
          style="display: flex; align-items: flex-start; gap: 10px; padding: 10px 12px; background: {hoveredIndex === index ? 'var(--bg-elevated)' : 'var(--bg-primary)'}; border: 1px solid var(--border-default); border-radius: 8px; transition: all 0.12s ease;"
        >
          <!-- Rule number -->
          <span style="min-width: 22px; height: 22px; background: rgba(99,102,241,0.15); color: var(--accent-indigo); border-radius: 50%; display: flex; align-items: center; justify-content: center; font-size: 11px; font-weight: 700; flex-shrink: 0; margin-top: 1px;">
            {index + 1}
          </span>

          {#if editingIndex === index}
            <!-- Edit mode -->
            <textarea
              bind:value={editingText}
              onkeydown={(e) => handleEditKeyDown(e, index)}
              rows="2"
              style="flex: 1; padding: 6px 8px; background: var(--bg-surface); border: 1px solid var(--accent-indigo); border-radius: 4px; color: var(--text-primary); font-size: 13px; font-family: var(--font-ui); outline: none; resize: vertical;"
            ></textarea>
            <div style="display: flex; flex-direction: column; gap: 4px;">
              <button
                onclick={() => saveEdit(index)}
                style="padding: 4px 8px; background: #3fb950; color: white; border: none; border-radius: 4px; font-size: 11px; cursor: pointer;"
              >Save</button>
              <button
                onclick={cancelEdit}
                style="padding: 4px 8px; background: var(--bg-elevated); color: var(--text-secondary); border: 1px solid var(--border-default); border-radius: 4px; font-size: 11px; cursor: pointer;"
              >Cancel</button>
            </div>
          {:else}
            <!-- Display mode -->
            <span style="flex: 1; font-size: 13px; color: var(--text-primary); line-height: 1.5; white-space: pre-wrap;">
              {rule}
            </span>

            <!-- Actions (visible on hover) -->
            <div style="display: flex; gap: 2px; opacity: {hoveredIndex === index ? 1 : 0}; transition: opacity 0.12s ease; flex-shrink: 0;">
              <button
                onclick={() => moveRule(index, -1)}
                title="Move up"
                disabled={index === 0}
                style="padding: 4px; background: none; border: none; cursor: {index === 0 ? 'default' : 'pointer'}; color: {index === 0 ? 'var(--text-muted)' : 'var(--text-secondary)'}; opacity: {index === 0 ? 0.3 : 1};"
              >
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M18 15l-6-6-6 6"/></svg>
              </button>
              <button
                onclick={() => moveRule(index, 1)}
                title="Move down"
                disabled={index === settingsStore.value.workerRules.length - 1}
                style="padding: 4px; background: none; border: none; cursor: {index === settingsStore.value.workerRules.length - 1 ? 'default' : 'pointer'}; color: {index === settingsStore.value.workerRules.length - 1 ? 'var(--text-muted)' : 'var(--text-secondary)'}; opacity: {index === settingsStore.value.workerRules.length - 1 ? 0.3 : 1};"
              >
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M6 9l6 6 6-6"/></svg>
              </button>
              <button
                onclick={() => startEdit(index)}
                title="Edit"
                style="padding: 4px; background: none; border: none; cursor: pointer; color: var(--text-secondary);"
              >
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z"/></svg>
              </button>
              <button
                onclick={() => removeRule(index)}
                title="Delete"
                style="padding: 4px; background: none; border: none; cursor: pointer; color: #f85149;"
              >
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 6h18M8 6V4h8v2m1 0v14a2 2 0 01-2 2H9a2 2 0 01-2-2V6"/></svg>
              </button>
            </div>
          {/if}
        </div>
      {/each}
    </div>

    <div style="font-size: 11px; color: var(--text-muted); text-align: right;">
      {settingsStore.value.workerRules.length} rule{settingsStore.value.workerRules.length === 1 ? '' : 's'}
    </div>
  {/if}
</div>
