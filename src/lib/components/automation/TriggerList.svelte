<script lang="ts">
	import type { AeTrigger } from '$lib/types';
	import { getAutomationStore } from '$lib/stores/automation.svelte';
	import { formatTimeAgo } from '$lib/utils/relative-time';

	interface Props {
		onEdit?: (trigger: AeTrigger) => void;
		onAdd?: () => void;
	}

	let { onEdit, onAdd }: Props = $props();
	const automation = getAutomationStore();

	const sourceTypeIcons: Record<string, string> = {
		supabase: 'DB',
		webhook: 'WH',
		github: 'GH',
		triage: 'TR',
	};

	const sourceTypeColors: Record<string, string> = {
		supabase: '#3fb950',
		webhook: '#6366f1',
		github: '#8b949e',
		triage: '#f59e0b',
	};
</script>

<div style="display: flex; flex-direction: column; gap: 6px;">
	<!-- Header -->
	<div style="display: flex; align-items: center; gap: 6px; padding: 0 2px;">
		<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="var(--accent-amber)" stroke-width="2">
			<polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"/>
		</svg>
		<span style="font-size: 11px; font-weight: 700; color: var(--text-secondary); text-transform: uppercase; letter-spacing: 0.5px; flex: 1;">
			Triggers
		</span>
		<button
			style="
				width: 20px; height: 20px; display: flex; align-items: center; justify-content: center;
				border: 1px solid var(--border-default); border-radius: 5px;
				background: transparent; color: var(--text-muted); cursor: pointer;
				transition: all 0.15s ease; font-size: 12px;
			"
			onmouseenter={(e) => { const t = e.currentTarget as HTMLElement; t.style.borderColor = 'rgba(99,102,241,0.4)'; t.style.color = 'var(--accent-indigo)'; t.style.transform = 'scale(1.1)'; }}
			onmouseleave={(e) => { const t = e.currentTarget as HTMLElement; t.style.borderColor = 'var(--border-default)'; t.style.color = 'var(--text-muted)'; t.style.transform = 'scale(1)'; }}
			onclick={() => onAdd?.()}
			title="Add Trigger"
		>
			+
		</button>
	</div>

	<!-- Trigger items -->
	{#each automation.triggers as trigger (trigger.id)}
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<div
			style="
				padding: 8px 10px; border-radius: 8px;
				background: var(--glass-bg); backdrop-filter: blur(var(--glass-blur));
				border: 1px solid var(--glass-border);
				cursor: pointer; transition: all 0.15s ease;
			"
			onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.borderColor = 'rgba(99,102,241,0.15)'; (e.currentTarget as HTMLElement).style.background = 'rgba(99,102,241,0.04)'; }}
			onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.borderColor = 'var(--glass-border)'; (e.currentTarget as HTMLElement).style.background = 'var(--glass-bg)'; }}
			onclick={() => onEdit?.(trigger)}
		>
			<div style="display: flex; align-items: center; gap: 6px;">
				<!-- Source type badge -->
				<span style="
					font-size: 8px; font-weight: 800; padding: 2px 5px; border-radius: 3px;
					background: {sourceTypeColors[trigger.source_type] || '#6e7681'}20;
					color: {sourceTypeColors[trigger.source_type] || '#6e7681'};
					font-family: var(--font-mono);
				">
					{sourceTypeIcons[trigger.source_type] || '??'}
				</span>

				<span style="font-size: 12px; font-weight: 600; color: var(--text-primary); flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
					{trigger.name}
				</span>

				<!-- Toggle -->
				<button
					style="
						width: 32px; height: 18px; border-radius: 9px;
						background: {trigger.enabled ? 'rgba(99,102,241,0.3)' : 'var(--bg-primary)'};
						border: 1px solid {trigger.enabled ? 'rgba(99,102,241,0.5)' : 'var(--border-default)'};
						cursor: pointer; position: relative; transition: all 0.2s ease;
						flex-shrink: 0;
					"
					onclick={(e) => { e.stopPropagation(); automation.toggleTrigger(trigger.id); }}
				>
					<span style="
						width: 12px; height: 12px; border-radius: 50%;
						background: {trigger.enabled ? 'var(--accent-indigo)' : 'var(--text-muted)'};
						position: absolute; top: 2px;
						left: {trigger.enabled ? '16px' : '2px'};
						transition: all 0.2s ease;
						box-shadow: {trigger.enabled ? '0 0 6px rgba(99,102,241,0.4)' : 'none'};
					"></span>
				</button>
			</div>
		</div>
	{/each}

	{#if automation.triggers.length === 0}
		<div style="padding: 12px; text-align: center; color: var(--text-muted); font-size: 11px; opacity: 0.6;">
			No triggers configured
		</div>
	{/if}
</div>
