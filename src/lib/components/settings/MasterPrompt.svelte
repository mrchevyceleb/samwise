<script lang="ts">
  interface Props {
    onConfirm: () => void;
    onDecline: () => void;
  }
  let { onConfirm, onDecline }: Props = $props();

  let confirmHovered = $state(false);
  let declineHovered = $state(false);
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  onclick={() => { /* backdrop click is a no-op - must choose a button */ }}
  style="position: fixed; inset: 0; z-index: 2000; background: rgba(0,0,0,0.7); backdrop-filter: blur(6px); display: flex; align-items: center; justify-content: center;"
>
  <div style="width: 420px; max-width: 90vw; background: var(--bg-surface, #1c1c21); border: 1px solid var(--border-default, #26262d); border-radius: 16px; box-shadow: 0 32px 80px rgba(0,0,0,0.6); overflow: hidden;">

    <!-- Content -->
    <div style="padding: 36px 32px 28px; display: flex; flex-direction: column; align-items: center; gap: 20px;">

      <!-- Sam icon with bobbing animation -->
      <div style="width: 72px; height: 72px; background: linear-gradient(135deg, #6366f1, #8b5cf6); border-radius: 18px; display: flex; align-items: center; justify-content: center; animation: bob 3s ease-in-out infinite;">
        <svg width="36" height="36" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.5">
          <circle cx="12" cy="8" r="5"/><path d="M3 21v-2a7 7 0 0 1 7-7h4a7 7 0 0 1 7 7v2"/><circle cx="9" cy="7" r="1" fill="white"/><circle cx="15" cy="7" r="1" fill="white"/>
        </svg>
      </div>

      <!-- Headline -->
      <div style="text-align: center;">
        <div style="font-size: 20px; font-weight: 700; color: var(--text-primary, #e2e6ed);">Is this Sam's home machine?</div>
        <div style="font-size: 13px; color: var(--text-secondary, #8f97a4); margin-top: 8px; line-height: 1.5;">Sam works on one machine and you can view his board and chat from anywhere else.</div>
      </div>

      <!-- Buttons -->
      <div style="display: flex; gap: 12px; width: 100%; margin-top: 4px;">
        <button
          onclick={onDecline}
          onmouseenter={() => declineHovered = true}
          onmouseleave={() => declineHovered = false}
          style="flex: 1; padding: 10px 16px; background: transparent; border: 1px solid var(--border-default, #26262d); border-radius: 8px; color: var(--text-secondary, #8f97a4); font-size: 13px; font-weight: 600; font-family: var(--font-ui); cursor: pointer; transition: all 0.15s ease;
            transform: {declineHovered ? 'scale(1.04)' : 'scale(1)'};
            background: {declineHovered ? 'var(--bg-elevated, #232329)' : 'transparent'};"
        >
          No, just viewing
        </button>
        <button
          onclick={onConfirm}
          onmouseenter={() => confirmHovered = true}
          onmouseleave={() => confirmHovered = false}
          style="flex: 1; padding: 10px 16px; background: #6366f1; border: none; border-radius: 8px; color: white; font-size: 13px; font-weight: 700; font-family: var(--font-ui); cursor: pointer; transition: all 0.15s ease;
            transform: {confirmHovered ? 'scale(1.04)' : 'scale(1)'};
            box-shadow: {confirmHovered ? '0 0 20px rgba(99,102,241,0.4)' : 'none'};"
        >
          Yes, this is home
        </button>
      </div>
    </div>
  </div>
</div>

<style>
  @keyframes bob {
    0%, 100% { transform: translateY(0); }
    50% { transform: translateY(-5px); }
  }
</style>
