<script lang="ts">
	import type { AeTrigger, TriggerSourceType } from '$lib/types';
	import { getAutomationStore } from '$lib/stores/automation.svelte';

	interface Props {
		trigger?: AeTrigger | null;
		onClose: () => void;
	}

	let { trigger = null, onClose }: Props = $props();
	const automation = getAutomationStore();

	let name = $state(trigger?.name || '');
	let sourceType = $state<TriggerSourceType>(trigger?.source_type || 'webhook');
	let sourceConfig = $state(trigger?.source_config ? JSON.stringify(trigger.source_config, null, 2) : '{}');
	let taskTitle = $state((trigger?.task_template as any)?.title || '');
	let taskPriority = $state((trigger?.task_template as any)?.priority || 'medium');
	let enabled = $state(trigger?.enabled ?? true);
	let saving = $state(false);

	const sourceTypes: { value: TriggerSourceType; label: string; desc: string }[] = [
		{ value: 'supabase', label: 'Supabase', desc: 'Table change' },
		{ value: 'webhook', label: 'Webhook', desc: 'HTTP endpoint' },
		{ value: 'github', label: 'GitHub', desc: 'Repo event' },
		{ value: 'triage', label: 'Triage', desc: 'Bug ticket' },
	];

	async function handleSave() {
		if (!name.trim() || saving) return;
		saving = true;
		try {
			let config: Record<string, unknown> = {};
			try { config = JSON.parse(sourceConfig); } catch {}

			const data = {
				name: name.trim(),
				source_type: sourceType,
				source_config: config,
				task_template: { title: taskTitle || name, priority: taskPriority },
				enabled,
			};

			if (trigger) {
				await automation.updateTrigger(trigger.id, data);
			} else {
				await automation.createTrigger(data);
			}
			onClose();
		} finally {
			saving = false;
		}
	}
</script>

<div style="display: flex; flex-direction: column; gap: 12px; padding: 12px; background: var(--bg-primary); border-radius: 10px; border: 1px solid var(--border-glow); animation: spring-in 0.2s ease;">
	<div style="font-size: 13px; font-weight: 700; color: var(--text-primary);">
		{trigger ? 'Edit' : 'New'} Trigger
	</div>

	<!-- Name -->
	<input
		type="text"
		bind:value={name}
		placeholder="Trigger name"
		style="padding: 8px 10px; background: var(--bg-surface); border: 1px solid var(--border-default); border-radius: 6px; color: var(--text-primary); font-size: 12px; font-family: var(--font-ui); outline: none;"
	/>

	<!-- Source type -->
	<div style="display: flex; gap: 4px;">
		{#each sourceTypes as st}
			<button
				style="
					flex: 1; padding: 4px 6px; border-radius: 6px; font-size: 10px;
					border: 1px solid {sourceType === st.value ? 'rgba(99,102,241,0.4)' : 'var(--border-default)'};
					background: {sourceType === st.value ? 'rgba(99,102,241,0.1)' : 'var(--bg-surface)'};
					color: {sourceType === st.value ? 'var(--accent-indigo)' : 'var(--text-muted)'};
					cursor: pointer; font-family: var(--font-ui); font-weight: 600; transition: all 0.15s;
				"
				onclick={() => sourceType = st.value}
			>
				{st.label}
			</button>
		{/each}
	</div>

	<!-- Source config -->
	<textarea
		bind:value={sourceConfig}
		placeholder="Source configuration (JSON)"
		rows={3}
		style="padding: 8px 10px; background: var(--bg-surface); border: 1px solid var(--border-default); border-radius: 6px; color: var(--text-primary); font-size: 11px; font-family: var(--font-mono); outline: none; resize: vertical;"
	></textarea>

	<!-- Task template -->
	<input
		type="text"
		bind:value={taskTitle}
		placeholder="Task title template"
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
			style="padding: 6px 12px; border: 1px solid var(--border-default); background: none; border-radius: 6px; color: var(--text-muted); font-size: 11px; cursor: pointer; font-family: var(--font-ui); transition: all 0.15s;"
			onclick={onClose}
		>
			Cancel
		</button>
		<button
			style="
				padding: 6px 16px; border: none; border-radius: 6px;
				background: {!name.trim() ? 'var(--text-muted)' : 'var(--accent-indigo)'};
				color: white; font-size: 11px; font-weight: 700; cursor: pointer;
				font-family: var(--font-ui); transition: all 0.15s;
			"
			onclick={handleSave}
			disabled={!name.trim() || saving}
		>
			{saving ? 'Saving...' : trigger ? 'Update' : 'Create'}
		</button>
	</div>
</div>
