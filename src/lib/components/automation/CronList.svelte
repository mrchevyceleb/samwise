<script lang="ts">
	import type { AeCron } from '$lib/types';
	import { getAutomationStore } from '$lib/stores/automation.svelte';

	interface Props {
		onEdit?: (cron: AeCron) => void;
		onAdd?: () => void;
	}

	let { onEdit, onAdd }: Props = $props();
	const automation = getAutomationStore();

	function humanSchedule(cron: string): string {
		// Simple cron-to-human for common patterns
		if (cron === '0 * * * *') return 'Every hour';
		if (cron === '*/30 * * * *') return 'Every 30 min';
		if (cron === '*/15 * * * *') return 'Every 15 min';
		if (cron === '0 9 * * *') return 'Daily at 9am';
		if (cron === '0 9 * * 1') return 'Every Monday at 9am';
		if (cron === '0 0 * * *') return 'Daily at midnight';
		return cron;
	}

	function formatDate(dateStr: string | null): string {
		if (!dateStr) return 'never';
		const d = new Date(dateStr);
		return d.toLocaleString([], { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' });
	}
</script>

<div style="display: flex; flex-direction: column; gap: 6px;">
	<!-- Header -->
	<div style="display: flex; align-items: center; gap: 6px; padding: 0 2px;">
		<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="var(--accent-blue)" stroke-width="2">
			<circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/>
		</svg>
		<span style="font-size: 11px; font-weight: 700; color: var(--text-secondary); text-transform: uppercase; letter-spacing: 0.5px; flex: 1;">
			Scheduled Jobs
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
			title="Add Cron Job"
		>
			+
		</button>
	</div>

	<!-- Cron items -->
	{#each automation.crons as cron (cron.id)}
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
			onclick={() => onEdit?.(cron)}
		>
			<div style="display: flex; align-items: center; gap: 6px; margin-bottom: 4px;">
				<span style="font-size: 12px; font-weight: 600; color: var(--text-primary); flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
					{cron.name}
				</span>

				<!-- Toggle -->
				<button
					style="
						width: 32px; height: 18px; border-radius: 9px;
						background: {cron.enabled ? 'rgba(99,102,241,0.3)' : 'var(--bg-primary)'};
						border: 1px solid {cron.enabled ? 'rgba(99,102,241,0.5)' : 'var(--border-default)'};
						cursor: pointer; position: relative; transition: all 0.2s ease;
						flex-shrink: 0;
					"
					onclick={(e) => { e.stopPropagation(); automation.toggleCron(cron.id); }}
				>
					<span style="
						width: 12px; height: 12px; border-radius: 50%;
						background: {cron.enabled ? 'var(--accent-indigo)' : 'var(--text-muted)'};
						position: absolute; top: 2px;
						left: {cron.enabled ? '16px' : '2px'};
						transition: all 0.2s ease;
						box-shadow: {cron.enabled ? '0 0 6px rgba(99,102,241,0.4)' : 'none'};
					"></span>
				</button>
			</div>

			<div style="display: flex; align-items: center; gap: 8px; font-size: 10px; color: var(--text-muted);">
				<span style="
					padding: 1px 5px; border-radius: 4px;
					background: rgba(88, 166, 255, 0.08);
					color: var(--accent-blue); font-family: var(--font-mono);
				">
					{humanSchedule(cron.schedule)}
				</span>
				<span>Next: {formatDate(cron.next_run)}</span>
			</div>
		</div>
	{/each}

	{#if automation.crons.length === 0}
		<div style="padding: 12px; text-align: center; color: var(--text-muted); font-size: 11px; opacity: 0.6;">
			No scheduled jobs
		</div>
	{/if}
</div>
