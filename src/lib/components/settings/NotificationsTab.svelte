<script lang="ts">
  import { getSettingsStore, updateSetting } from '$lib/stores/settings.svelte';

  const settingsStore = getSettingsStore();

  let hoveredToggle = $state<string | null>(null);

  const events = [
    { key: 'telegramNotifyTaskStarted' as const, label: 'Task Started', desc: 'When the worker picks up a new task' },
    { key: 'telegramNotifyTaskCompletedCode' as const, label: 'Task Completed (Code)', desc: 'When a PR is created' },
    { key: 'telegramNotifyTaskCompletedResearch' as const, label: 'Task Completed (Research)', desc: 'When an analysis report is finished' },
    { key: 'telegramNotifyTaskFailed' as const, label: 'Task Failed', desc: 'When a task hits an error' },
  ];

  function toggle(key: typeof events[number]['key']) {
    updateSetting(key, !settingsStore.value[key]);
  }

  function toggleMaster() {
    updateSetting('telegramNotificationsEnabled', !settingsStore.value.telegramNotificationsEnabled);
  }

  let masterEnabled = $derived(settingsStore.value.telegramNotificationsEnabled);
</script>

<div style="display: flex; flex-direction: column; gap: 20px;">
  <div style="font-size: 13px; font-weight: 600; color: var(--text-primary); padding-bottom: 4px; border-bottom: 1px solid var(--border-default);">
    Telegram Notifications
  </div>

  <div style="font-size: 12px; color: var(--text-secondary); line-height: 1.5;">
    Control which events send a Telegram message. Requires TELEGRAM_BOT_TOKEN and TELEGRAM_CHAT_ID in Doppler.
    Chat replies always send regardless of these settings.
  </div>

  <!-- Master toggle -->
  <div
    style="display: flex; align-items: center; justify-content: space-between; padding: 14px 16px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 10px;"
    onmouseenter={() => hoveredToggle = 'master'}
    onmouseleave={() => hoveredToggle = null}
  >
    <div>
      <div style="font-size: 13px; font-weight: 600; color: var(--text-primary);">Enable Notifications</div>
      <div style="font-size: 11px; color: var(--text-muted); margin-top: 2px;">Master switch for all task notifications</div>
    </div>
    <button
      onclick={toggleMaster}
      style="
        position: relative; width: 44px; height: 24px; border-radius: 12px; cursor: pointer;
        background: {masterEnabled ? '#6366f1' : 'var(--bg-elevated)'};
        border: 1px solid {masterEnabled ? '#6366f1' : 'var(--border-default)'};
        transition: all 0.2s ease;
        transform: {hoveredToggle === 'master' ? 'scale(1.05)' : 'scale(1)'};
      "
    >
      <span style="
        position: absolute; top: 2px; left: {masterEnabled ? '22px' : '2px'};
        width: 18px; height: 18px; border-radius: 50%;
        background: {masterEnabled ? 'white' : 'var(--text-muted)'};
        transition: all 0.2s ease;
        box-shadow: 0 1px 3px rgba(0,0,0,0.3);
      "></span>
    </button>
  </div>

  <!-- Individual event toggles -->
  <div style="display: flex; flex-direction: column; gap: 6px; opacity: {masterEnabled ? 1 : 0.4}; transition: opacity 0.2s ease;">
    {#each events as event (event.key)}
      <div
        style="display: flex; align-items: center; justify-content: space-between; padding: 12px 16px; background: {hoveredToggle === event.key ? 'var(--bg-elevated)' : 'var(--bg-primary)'}; border: 1px solid var(--border-default); border-radius: 8px; transition: all 0.12s ease;"
        onmouseenter={() => hoveredToggle = event.key}
        onmouseleave={() => hoveredToggle = null}
      >
        <div>
          <div style="font-size: 13px; font-weight: 500; color: var(--text-primary);">{event.label}</div>
          <div style="font-size: 11px; color: var(--text-muted); margin-top: 2px;">{event.desc}</div>
        </div>
        <button
          onclick={() => toggle(event.key)}
          disabled={!masterEnabled}
          style="
            position: relative; width: 40px; height: 22px; border-radius: 11px;
            cursor: {masterEnabled ? 'pointer' : 'default'};
            background: {settingsStore.value[event.key] ? '#6366f1' : 'var(--bg-elevated)'};
            border: 1px solid {settingsStore.value[event.key] ? '#6366f1' : 'var(--border-default)'};
            transition: all 0.2s ease;
            transform: {hoveredToggle === event.key && masterEnabled ? 'scale(1.05)' : 'scale(1)'};
          "
        >
          <span style="
            position: absolute; top: 2px; left: {settingsStore.value[event.key] ? '20px' : '2px'};
            width: 16px; height: 16px; border-radius: 50%;
            background: {settingsStore.value[event.key] ? 'white' : 'var(--text-muted)'};
            transition: all 0.2s ease;
            box-shadow: 0 1px 3px rgba(0,0,0,0.3);
          "></span>
        </button>
      </div>
    {/each}
  </div>

  <div style="font-size: 11px; color: var(--text-muted); text-align: right;">
    {#if masterEnabled}
      {events.filter(e => settingsStore.value[e.key]).length} of {events.length} enabled
    {:else}
      All notifications paused
    {/if}
  </div>
</div>
