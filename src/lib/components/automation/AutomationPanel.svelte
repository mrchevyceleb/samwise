<script lang="ts">
	import { onMount } from 'svelte';
	import type { AeTrigger, AeCron } from '$lib/types';
	import { getAutomationStore } from '$lib/stores/automation.svelte';
	import { getWorkerStore } from '$lib/stores/worker.svelte';
	import TriggerList from './TriggerList.svelte';
	import CronList from './CronList.svelte';
	import TriggerForm from './TriggerForm.svelte';
	import CronForm from './CronForm.svelte';

	const automation = getAutomationStore();
	const worker = getWorkerStore();

	let editingTrigger = $state<AeTrigger | null>(null);
	let showNewTrigger = $state(false);
	let editingCron = $state<AeCron | null>(null);
	let showNewCron = $state(false);

	onMount(() => {
		automation.fetchTriggers();
		automation.fetchCrons();
		worker.fetchStatus();
	});
</script>

<div style="display: flex; flex-direction: column; height: 100%; overflow: hidden;">
	<!-- Header -->
	<div style="
		display: flex; align-items: center; gap: 8px;
		padding: 10px 12px; flex-shrink: 0;
		border-bottom: 1px solid var(--border-subtle);
	">
		<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="var(--accent-indigo)" stroke-width="2" stroke-linecap="round">
			<circle cx="12" cy="12" r="3"/><path d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"/>
		</svg>
		<span style="font-size: 12px; font-weight: 700; color: var(--text-primary); letter-spacing: -0.2px;">Automation</span>
	</div>

	<!-- Worker status card -->
	<div style="padding: 8px 10px; flex-shrink: 0;">
		<div style="
			padding: 10px 12px; border-radius: 10px;
			background: var(--glass-bg); backdrop-filter: blur(var(--glass-blur));
			border: 1px solid var(--glass-border);
		">
			<div style="display: flex; align-items: center; gap: 8px; margin-bottom: 6px;">
				<div style="
					width: 28px; height: 28px; border-radius: 8px;
					background: rgba(99, 102, 241, 0.12);
					display: flex; align-items: center; justify-content: center;
					{worker.isBusy ? 'animation: pulse-ring 2s ease-out infinite;' : ''}
				">
					<!-- Robot face -->
					<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="var(--accent-indigo)" stroke-width="2" stroke-linecap="round">
						<rect x="4" y="6" width="16" height="14" rx="2"/><line x1="9" y1="1" x2="9" y2="6"/><line x1="15" y1="1" x2="15" y2="6"/><circle cx="9" cy="13" r="1" fill="var(--accent-indigo)"/><circle cx="15" cy="13" r="1" fill="var(--accent-indigo)"/>
					</svg>
				</div>
				<div style="flex: 1;">
					<div style="font-size: 12px; font-weight: 700; color: var(--text-primary);">AI Worker</div>
					<div style="font-size: 10px; color: var(--text-muted);">{worker.machineName}</div>
				</div>
				<div style="display: flex; align-items: center; gap: 4px;">
					<span style="
						width: 8px; height: 8px; border-radius: 50%;
						background: {worker.statusColor};
						box-shadow: 0 0 6px {worker.statusColor}60;
						{worker.status !== 'offline' ? 'animation: pulse-dot 2s ease-in-out infinite;' : ''}
					"></span>
					<span style="font-size: 11px; font-weight: 600; color: {worker.statusColor};">{worker.statusLabel}</span>
				</div>
			</div>

			{#if worker.currentTask}
				<div style="font-size: 11px; color: var(--text-secondary); padding: 4px 8px; background: rgba(99,102,241,0.05); border-radius: 6px; border: 1px solid rgba(99,102,241,0.1);">
					Working on: <span style="font-weight: 600; color: var(--text-primary);">{worker.currentTask.title}</span>
				</div>
			{/if}

			<div style="display: flex; gap: 6px; margin-top: 8px;">
				{#if worker.isOnline}
					<button
						style="
							flex: 1; padding: 5px; border-radius: 6px;
							border: 1px solid rgba(248, 81, 73, 0.3); background: rgba(248, 81, 73, 0.08);
							color: var(--accent-red); font-size: 11px; font-weight: 600;
							cursor: pointer; font-family: var(--font-ui); transition: all 0.15s;
						"
						onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = 'rgba(248, 81, 73, 0.15)'; }}
						onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background = 'rgba(248, 81, 73, 0.08)'; }}
						onclick={() => worker.stopWorker()}
					>
						Stop Worker
					</button>
				{:else}
					<button
						style="
							flex: 1; padding: 5px; border-radius: 6px;
							border: 1px solid rgba(99, 102, 241, 0.3); background: rgba(99, 102, 241, 0.08);
							color: var(--accent-indigo); font-size: 11px; font-weight: 600;
							cursor: pointer; font-family: var(--font-ui); transition: all 0.15s;
						"
						onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = 'rgba(99, 102, 241, 0.15)'; }}
						onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background = 'rgba(99, 102, 241, 0.08)'; }}
						onclick={() => worker.startWorker()}
					>
						Start Worker
					</button>
				{/if}
			</div>
		</div>
	</div>

	<!-- Content -->
	<div style="flex: 1; overflow-y: auto; padding: 6px 10px; display: flex; flex-direction: column; gap: 16px;">
		<!-- Triggers -->
		{#if showNewTrigger || editingTrigger}
			<TriggerForm
				trigger={editingTrigger}
				onClose={() => { showNewTrigger = false; editingTrigger = null; }}
			/>
		{:else}
			<TriggerList
				onEdit={(t) => editingTrigger = t}
				onAdd={() => showNewTrigger = true}
			/>
		{/if}

		<!-- Divider -->
		<div style="height: 1px; background: var(--border-subtle);"></div>

		<!-- Crons -->
		{#if showNewCron || editingCron}
			<CronForm
				cron={editingCron}
				onClose={() => { showNewCron = false; editingCron = null; }}
			/>
		{:else}
			<CronList
				onEdit={(c) => editingCron = c}
				onAdd={() => showNewCron = true}
			/>
		{/if}
	</div>
</div>
