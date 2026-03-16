<script lang="ts">
	import type { AeCron } from '$lib/types';
	import { getAutomationStore } from '$lib/stores/automation.svelte';

	interface Props {
		cron?: AeCron | null;
		onClose: () => void;
	}

	let { cron = null, onClose }: Props = $props();
	const automation = getAutomationStore();

	let name = $state(cron?.name || '');
	let scheduleMode = $state<'preset' | 'custom'>('preset');
	let preset = $state('0 * * * *');
	let customSchedule = $state(cron?.schedule || '');
	let taskTitle = $state((cron?.task_template as any)?.title || '');
	let enabled = $state(cron?.enabled ?? true);
	let saving = $state(false);

	const presets = [
		{ value: '*/15 * * * *', label: 'Every 15 min' },
		{ value: '*/30 * * * *', label: 'Every 30 min' },
		{ value: '0 * * * *', label: 'Every hour' },
		{ value: '0 9 * * *', label: 'Daily at 9am' },
		{ value: '0 9 * * 1', label: 'Every Monday' },
		{ value: '0 0 * * *', label: 'Daily at midnight' },
		{ value: '0 9 * * 1-5', label: 'Weekdays at 9am' },
	];

	// Initialize schedule mode from existing cron
	$effect(() => {
		if (cron?.schedule) {
			const matchesPreset = presets.find(p => p.value === cron!.schedule);
			if (matchesPreset) {
				scheduleMode = 'preset';
				preset = matchesPreset.value;
			} else {
				scheduleMode = 'custom';
				customSchedule = cron!.schedule;
			}
		}
	});

	let schedule = $derived(scheduleMode === 'preset' ? preset : customSchedule);

	async function handleSave() {
		if (!name.trim() || !schedule || saving) return;
		saving = true;
		try {
			const data = {
				name: name.trim(),
				schedule,
				task_template: { title: taskTitle || name, priority: 'medium' },
				enabled,
			};

			if (cron) {
				await automation.updateCron(cron.id, data);
			} else {
				await automation.createCron(data);
			}
			onClose();
		} finally {
			saving = false;
		}
	}
</script>

<div style="display: flex; flex-direction: column; gap: 12px; padding: 12px; background: var(--bg-primary); border-radius: 10px; border: 1px solid var(--border-glow); animation: spring-in 0.2s ease;">
	<div style="font-size: 13px; font-weight: 700; color: var(--text-primary);">
		{cron ? 'Edit' : 'New'} Scheduled Job
	</div>

	<!-- Name -->
	<input
		type="text"
		bind:value={name}
		placeholder="Job name"
		style="padding: 8px 10px; background: var(--bg-surface); border: 1px solid var(--border-default); border-radius: 6px; color: var(--text-primary); font-size: 12px; font-family: var(--font-ui); outline: none;"
	/>

	<!-- Schedule mode toggle -->
	<div style="display: flex; gap: 4px;">
		<button
			style="flex: 1; padding: 5px; border-radius: 6px; font-size: 11px; border: 1px solid {scheduleMode === 'preset' ? 'rgba(99,102,241,0.4)' : 'var(--border-default)'}; background: {scheduleMode === 'preset' ? 'rgba(99,102,241,0.1)' : 'var(--bg-surface)'}; color: {scheduleMode === 'preset' ? 'var(--accent-indigo)' : 'var(--text-muted)'}; cursor: pointer; font-family: var(--font-ui); font-weight: 600; transition: all 0.15s;"
			onclick={() => scheduleMode = 'preset'}
		>
			Presets
		</button>
		<button
			style="flex: 1; padding: 5px; border-radius: 6px; font-size: 11px; border: 1px solid {scheduleMode === 'custom' ? 'rgba(99,102,241,0.4)' : 'var(--border-default)'}; background: {scheduleMode === 'custom' ? 'rgba(99,102,241,0.1)' : 'var(--bg-surface)'}; color: {scheduleMode === 'custom' ? 'var(--accent-indigo)' : 'var(--text-muted)'}; cursor: pointer; font-family: var(--font-ui); font-weight: 600; transition: all 0.15s;"
			onclick={() => scheduleMode = 'custom'}
		>
			Custom Cron
		</button>
	</div>

	<!-- Schedule selector -->
	{#if scheduleMode === 'preset'}
		<div style="display: flex; flex-direction: column; gap: 3px;">
			{#each presets as p}
				<button
					style="
						padding: 6px 10px; border-radius: 6px; text-align: left;
						border: 1px solid {preset === p.value ? 'rgba(99,102,241,0.3)' : 'var(--border-subtle)'};
						background: {preset === p.value ? 'rgba(99,102,241,0.08)' : 'transparent'};
						color: {preset === p.value ? 'var(--accent-indigo)' : 'var(--text-secondary)'};
						font-size: 11px; cursor: pointer; font-family: var(--font-ui); transition: all 0.12s;
					"
					onclick={() => preset = p.value}
				>
					{p.label}
					<span style="float: right; font-family: var(--font-mono); font-size: 10px; color: var(--text-muted);">{p.value}</span>
				</button>
			{/each}
		</div>
	{:else}
		<input
			type="text"
			bind:value={customSchedule}
			placeholder="*/5 * * * * (cron expression)"
			style="padding: 8px 10px; background: var(--bg-surface); border: 1px solid var(--border-default); border-radius: 6px; color: var(--text-primary); font-size: 12px; font-family: var(--font-mono); outline: none;"
		/>
	{/if}

	<!-- Task template -->
	<input
		type="text"
		bind:value={taskTitle}
		placeholder="Task title when triggered"
		style="padding: 8px 10px; background: var(--bg-surface); border: 1px solid var(--border-default); border-radius: 6px; color: var(--text-primary); font-size: 12px; font-family: var(--font-ui); outline: none;"
	/>

	<!-- Enable toggle -->
	<div style="display: flex; align-items: center; gap: 8px;">
		<span style="font-size: 11px; color: var(--text-secondary); flex: 1;">Enabled</span>
		<button
			style="
				width: 36px; height: 20px; border-radius: 10px;
				background: {enabled ? 'rgba(99,102,241,0.3)' : 'var(--bg-surface)'};
				border: 1px solid {enabled ? 'rgba(99,102,241,0.5)' : 'var(--border-default)'};
				cursor: pointer; position: relative; transition: all 0.2s ease;
			"
			onclick={() => enabled = !enabled}
		>
			<span style="
				width: 14px; height: 14px; border-radius: 50%;
				background: {enabled ? 'var(--accent-indigo)' : 'var(--text-muted)'};
				position: absolute; top: 2px;
				left: {enabled ? '18px' : '2px'};
				transition: all 0.2s ease;
			"></span>
		</button>
	</div>

	<!-- Actions -->
	<div style="display: flex; gap: 6px; justify-content: flex-end;">
		<button
			style="padding: 6px 12px; border: 1px solid var(--border-default); background: none; border-radius: 6px; color: var(--text-muted); font-size: 11px; cursor: pointer; font-family: var(--font-ui);"
			onclick={onClose}
		>
			Cancel
		</button>
		<button
			style="padding: 6px 16px; border: none; border-radius: 6px; background: {!name.trim() ? 'var(--text-muted)' : 'var(--accent-indigo)'}; color: white; font-size: 11px; font-weight: 700; cursor: pointer; font-family: var(--font-ui);"
			onclick={handleSave}
			disabled={!name.trim() || saving}
		>
			{saving ? 'Saving...' : cron ? 'Update' : 'Create'}
		</button>
	</div>
</div>
