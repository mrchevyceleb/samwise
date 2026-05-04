<script lang="ts">
	import { onMount } from 'svelte';
	import type { AeCron, AeTrigger, TaskPriority, TaskType } from '$lib/types';
	import { getAutomationStore } from '$lib/stores/automation.svelte';
	import { getProjectStore } from '$lib/stores/projects.svelte';
	import { getSettings } from '$lib/stores/settings.svelte';

	const automation = getAutomationStore();
	const projectStore = getProjectStore();

	let apiBase = $derived.by(() => {
		const settings = getSettings();
		const url = settings.supabaseUrl || 'https://iycloielqcjnjqddeuet.supabase.co';
		return `${url}/functions/v1/agent-api`;
	});

	// ── State ──────────────────────────────────────────────────
	let activeSection = $state<'crons' | 'webhooks'>('crons');
	let showCronForm = $state(false);
	let editingCron = $state<AeCron | null>(null);
	let showWebhookForm = $state(false);

	// Cron form fields — minimal: prompt + repo target + schedule + enabled.
	// Name and title are derived from the prompt's first line.
	let cronPrompt = $state('');
	let cronSchedule = $state('0 9 * * *');
	let cronScheduleMode = $state<'preset' | 'custom'>('preset');
	let cronCustom = $state('');
	let cronRepoMode = $state<'project' | 'parent'>('project');
	let cronProject = $state('');
	let cronRepoParent = $state('');
	let cronEnabled = $state(true);
	let cronSaving = $state(false);

	let cronRepoOk = $derived(cronRepoMode === 'project' ? !!cronProject : !!cronRepoParent.trim());
	let cronCanSave = $derived(!!cronPrompt.trim() && cronRepoOk && !cronSaving);

	// Webhook form fields
	let webhookName = $state('');
	let webhookTaskTitle = $state('');
	let webhookTaskType = $state<TaskType>('code');
	let webhookPriority = $state<TaskPriority>('medium');
	let webhookSaving = $state(false);

	// Clipboard feedback
	let copiedId = $state<string | null>(null);

	// Presets use UTC values that correspond to EST (UTC-5)
	const presets = [
		{ value: '*/15 * * * *', label: 'Every 15 min' },
		{ value: '*/30 * * * *', label: 'Every 30 min' },
		{ value: '0 * * * *', label: 'Every hour' },
		{ value: '0 14 * * *', label: 'Daily at 9am ET' },
		{ value: '0 14 * * 1-5', label: 'Weekdays at 9am ET' },
		{ value: '0 14 * * 1', label: 'Every Monday 9am ET' },
		{ value: '0 17 * * *', label: 'Daily at noon ET' },
		{ value: '0 5 * * *', label: 'Daily at midnight ET' },
		{ value: '0 */6 * * *', label: 'Every 6 hours' },
	];

	const priorities: { value: TaskPriority; label: string; color: string }[] = [
		{ value: 'critical', label: 'Critical', color: '#f85149' },
		{ value: 'high', label: 'High', color: '#d29922' },
		{ value: 'medium', label: 'Medium', color: '#6366f1' },
		{ value: 'low', label: 'Low', color: '#6e7681' },
	];

	onMount(() => {
		automation.fetchCrons();
		automation.fetchTriggers();
		projectStore.fetchProjects();
	});

	// ── Cron helpers ───────────────────────────────────────────
	function resetCronForm() {
		cronPrompt = ''; cronSchedule = '0 9 * * *';
		cronScheduleMode = 'preset'; cronCustom = '';
		cronRepoMode = 'project'; cronProject = ''; cronRepoParent = '';
		cronEnabled = true; editingCron = null;
	}

	function deriveName(promptText: string): string {
		const firstLine = promptText.split('\n').map(s => s.trim()).find(s => s.length > 0) || 'Scheduled job';
		return firstLine.length > 80 ? firstLine.slice(0, 77) + '...' : firstLine;
	}

	function editCron(cron: AeCron) {
		editingCron = cron;
		const tpl = (cron.task_template ?? {}) as Record<string, any>;
		// Migrate legacy templates: prefer description, fall back to title, fall back to name
		cronPrompt = (tpl?.description as string) || (tpl?.title as string) || cron.name || '';
		cronProject = (tpl?.project as string) || '';
		cronRepoParent = (tpl?.repo_parent as string) || '';
		cronRepoMode = cronRepoParent ? 'parent' : 'project';
		cronEnabled = cron.enabled;
		const matchesPreset = presets.find(p => p.value === cron.schedule);
		if (matchesPreset) {
			cronScheduleMode = 'preset';
			cronSchedule = matchesPreset.value;
		} else {
			cronScheduleMode = 'custom';
			cronCustom = cron.schedule;
		}
		showCronForm = true;
	}

	async function saveCron() {
		const schedule = cronScheduleMode === 'preset' ? cronSchedule : cronCustom;
		const promptText = cronPrompt.trim();
		const repoOk = cronRepoMode === 'project' ? !!cronProject : !!cronRepoParent.trim();
		if (!promptText || !schedule || !repoOk || cronSaving) return;
		cronSaving = true;
		try {
			const derivedName = deriveName(promptText);
			const template: Record<string, unknown> = {
				title: derivedName,
				description: promptText,
				priority: 'medium',
			};
			if (cronRepoMode === 'project') {
				template.project = cronProject;
			} else {
				template.repo_parent = cronRepoParent.trim();
			}
			const data = {
				name: derivedName,
				schedule,
				task_template: template,
				enabled: cronEnabled,
			};
			if (editingCron) {
				await automation.updateCron(editingCron.id, data);
			} else {
				await automation.createCron(data);
			}
			showCronForm = false;
			resetCronForm();
		} finally {
			cronSaving = false;
		}
	}

	// ── Webhook helpers ────────────────────────────────────────
	async function createWebhook() {
		if (!webhookName.trim() || webhookSaving) return;
		webhookSaving = true;
		try {
			await automation.createTrigger({
				name: webhookName.trim(),
				source_type: 'webhook',
				source_config: {},
				task_template: {
					title: webhookTaskTitle || webhookName,
					task_type: webhookTaskType,
					priority: webhookPriority,
				},
			});
			showWebhookForm = false;
			webhookName = ''; webhookTaskTitle = ''; webhookTaskType = 'code'; webhookPriority = 'medium';
		} finally {
			webhookSaving = false;
		}
	}

	function webhookUrl(triggerId: string): string {
		return `${apiBase}/webhook/${triggerId}`;
	}

	async function copyToClipboard(text: string, id: string) {
		try {
			await navigator.clipboard.writeText(text);
			copiedId = id;
			setTimeout(() => { if (copiedId === id) copiedId = null; }, 2000);
		} catch {
			// Fallback for Tauri
			const el = document.createElement('textarea');
			el.value = text; document.body.appendChild(el);
			el.select(); document.execCommand('copy');
			document.body.removeChild(el);
			copiedId = id;
			setTimeout(() => { if (copiedId === id) copiedId = null; }, 2000);
		}
	}

	function humanSchedule(cron: string): string {
		const match = presets.find(p => p.value === cron);
		if (match) return match.label;
		// Also check the old CronList format
		const common: Record<string, string> = {
			'0 * * * *': 'Every hour',
			'*/15 * * * *': 'Every 15 min',
			'*/30 * * * *': 'Every 30 min',
			'0 14 * * *': 'Daily 9am ET',
			'0 14 * * 1-5': 'Weekdays 9am ET',
			'0 14 * * 1': 'Monday 9am ET',
			'0 17 * * *': 'Daily noon ET',
			'0 5 * * *': 'Daily midnight ET',
		};
		return common[cron] || cron;
	}

	function formatDate(dateStr: string | null): string {
		if (!dateStr) return 'never';
		return new Date(dateStr).toLocaleString([], { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' });
	}

	let webhookTriggers = $derived(automation.triggers.filter(t => t.source_type === 'webhook'));

	// Shared styles
	const inputStyle = 'padding: 8px 10px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 6px; color: var(--text-primary); font-size: 13px; font-family: var(--font-ui); outline: none; width: 100%; box-sizing: border-box;';
	const monoInputStyle = 'padding: 8px 10px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 6px; color: var(--text-primary); font-size: 12px; font-family: var(--font-mono, monospace); outline: none; width: 100%; box-sizing: border-box;';
	const labelStyle = 'font-size: 12px; color: var(--text-secondary); font-weight: 500;';
	const sectionTitle = 'font-size: 13px; font-weight: 600; color: var(--text-primary); padding-bottom: 4px; border-bottom: 1px solid var(--border-default);';
</script>

<div style="display: flex; flex-direction: column; gap: 20px;">
	<!-- Section tabs -->
	<div style="display: flex; gap: 4px; background: var(--bg-primary); border-radius: 8px; padding: 3px;">
		<button
			onclick={() => activeSection = 'crons'}
			style="flex: 1; padding: 6px 12px; border: none; border-radius: 6px; font-size: 12px; font-weight: 600; cursor: pointer; font-family: var(--font-ui); transition: all 0.15s;
				background: {activeSection === 'crons' ? 'rgba(99,102,241,0.12)' : 'transparent'};
				color: {activeSection === 'crons' ? 'var(--accent-indigo)' : 'var(--text-muted)'};"
		>
			Recurring Tasks
		</button>
		<button
			onclick={() => activeSection = 'webhooks'}
			style="flex: 1; padding: 6px 12px; border: none; border-radius: 6px; font-size: 12px; font-weight: 600; cursor: pointer; font-family: var(--font-ui); transition: all 0.15s;
				background: {activeSection === 'webhooks' ? 'rgba(99,102,241,0.12)' : 'transparent'};
				color: {activeSection === 'webhooks' ? 'var(--accent-indigo)' : 'var(--text-muted)'};"
		>
			Webhooks
		</button>
	</div>

	<!-- ═══ RECURRING TASKS ═══════════════════════════════════════ -->
	{#if activeSection === 'crons'}
		<div style="display: flex; flex-direction: column; gap: 16px;">
			<div style="display: flex; align-items: center;">
				<div style="{sectionTitle} flex: 1; border: none; padding: 0;">Scheduled Recurring Tasks</div>
				<button
					onclick={() => { resetCronForm(); showCronForm = true; }}
					style="padding: 5px 12px; border: 1px solid rgba(99,102,241,0.3); background: rgba(99,102,241,0.08); color: var(--accent-indigo); border-radius: 6px; font-size: 12px; font-weight: 600; cursor: pointer; font-family: var(--font-ui); transition: all 0.15s;"
				>
					+ New Recurring Task
				</button>
			</div>

			<div style="font-size: 12px; color: var(--text-muted); line-height: 1.5;">
				Recurring tasks run on a schedule. Sam picks them up automatically when the worker is online. Presets are Eastern Time. Custom cron expressions use UTC.
			</div>

			<!-- Cron form -->
			{#if showCronForm}
				<div style="padding: 16px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 10px; display: flex; flex-direction: column; gap: 12px;">
					<div style="font-size: 13px; font-weight: 700; color: var(--text-primary);">
						{editingCron ? 'Edit' : 'New'} Recurring Task
					</div>

					<!-- Prompt -->
					<div style="display: flex; flex-direction: column; gap: 4px;">
						<span style="{labelStyle}">Prompt</span>
						<textarea bind:value={cronPrompt} placeholder="What should Sam do? e.g. /match, or 'audit dependencies and report stale ones', or 'grab Railway logs and create triage tickets for critical issues'"
							style="padding: 8px 10px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 6px; color: var(--text-primary); font-size: 13px; font-family: var(--font-ui); outline: none; resize: vertical; min-height: 80px; width: 100%; box-sizing: border-box; line-height: 1.5;"></textarea>
					</div>

					<!-- Repo target -->
					<div style="display: flex; flex-direction: column; gap: 4px;">
						<span style="{labelStyle}">Repo</span>
						<div style="display: flex; gap: 4px; margin-bottom: 4px;">
							<button onclick={() => cronRepoMode = 'project'}
								style="padding: 4px 10px; border-radius: 5px; font-size: 11px; font-weight: 600; cursor: pointer; font-family: var(--font-ui); transition: all 0.15s;
									border: 1px solid {cronRepoMode === 'project' ? 'rgba(99,102,241,0.4)' : 'var(--border-default)'};
									background: {cronRepoMode === 'project' ? 'rgba(99,102,241,0.1)' : 'transparent'};
									color: {cronRepoMode === 'project' ? 'var(--accent-indigo)' : 'var(--text-muted)'};">
								Single project
							</button>
							<button onclick={() => cronRepoMode = 'parent'}
								style="padding: 4px 10px; border-radius: 5px; font-size: 11px; font-weight: 600; cursor: pointer; font-family: var(--font-ui); transition: all 0.15s;
									border: 1px solid {cronRepoMode === 'parent' ? 'rgba(99,102,241,0.4)' : 'var(--border-default)'};
									background: {cronRepoMode === 'parent' ? 'rgba(99,102,241,0.1)' : 'transparent'};
									color: {cronRepoMode === 'parent' ? 'var(--accent-indigo)' : 'var(--text-muted)'};">
								All repos in folder
							</button>
						</div>
						{#if cronRepoMode === 'project'}
							<select bind:value={cronProject}
								style="padding: 8px 10px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 6px; color: var(--text-primary); font-size: 13px; font-family: var(--font-ui); outline: none; cursor: pointer;">
								<option value="">Select a project…</option>
								{#each projectStore.projects as proj}
									<option value={proj.name}>{proj.name}</option>
								{/each}
							</select>
						{:else}
							<input bind:value={cronRepoParent} placeholder="/Users/mjohnst/samwise/Personal-Apps" style="{monoInputStyle}" />
							<div style="font-size: 10px; color: var(--text-muted); margin-top: 2px; line-height: 1.5;">
								Fans out at trigger time: one task per direct subfolder containing a .git directory.
							</div>
						{/if}
					</div>

					<!-- Schedule -->
					<div style="display: flex; flex-direction: column; gap: 4px;">
						<span style="{labelStyle}">Schedule</span>
						<div style="display: flex; gap: 4px; margin-bottom: 4px;">
							<button onclick={() => cronScheduleMode = 'preset'}
								style="padding: 4px 10px; border-radius: 5px; font-size: 11px; font-weight: 600; cursor: pointer; font-family: var(--font-ui); transition: all 0.15s;
									border: 1px solid {cronScheduleMode === 'preset' ? 'rgba(99,102,241,0.4)' : 'var(--border-default)'};
									background: {cronScheduleMode === 'preset' ? 'rgba(99,102,241,0.1)' : 'transparent'};
									color: {cronScheduleMode === 'preset' ? 'var(--accent-indigo)' : 'var(--text-muted)'};">
								Presets
							</button>
							<button onclick={() => cronScheduleMode = 'custom'}
								style="padding: 4px 10px; border-radius: 5px; font-size: 11px; font-weight: 600; cursor: pointer; font-family: var(--font-ui); transition: all 0.15s;
									border: 1px solid {cronScheduleMode === 'custom' ? 'rgba(99,102,241,0.4)' : 'var(--border-default)'};
									background: {cronScheduleMode === 'custom' ? 'rgba(99,102,241,0.1)' : 'transparent'};
									color: {cronScheduleMode === 'custom' ? 'var(--accent-indigo)' : 'var(--text-muted)'};">
								Custom
							</button>
						</div>
						{#if cronScheduleMode === 'preset'}
							<div style="display: grid; grid-template-columns: 1fr 1fr; gap: 3px;">
								{#each presets as p}
									<button
										onclick={() => cronSchedule = p.value}
										style="padding: 6px 10px; border-radius: 6px; text-align: left; font-size: 12px; cursor: pointer; font-family: var(--font-ui); transition: all 0.12s;
											border: 1px solid {cronSchedule === p.value ? 'rgba(99,102,241,0.3)' : 'var(--border-subtle)'};
											background: {cronSchedule === p.value ? 'rgba(99,102,241,0.08)' : 'transparent'};
											color: {cronSchedule === p.value ? 'var(--accent-indigo)' : 'var(--text-secondary)'};"
									>
										{p.label}
									</button>
								{/each}
							</div>
						{:else}
							<input bind:value={cronCustom} placeholder="*/5 * * * *" style="{monoInputStyle}" />
							<div style="font-size: 10px; color: var(--text-muted); margin-top: 2px;">
								Format: minute hour day month weekday (e.g. 0 9 * * 1-5 = weekdays at 9am)
							</div>
						{/if}
					</div>

					<!-- Enable + Actions -->
					<div style="display: flex; align-items: center; gap: 8px; padding-top: 4px; border-top: 1px solid var(--border-subtle);">
						<label style="display: flex; align-items: center; gap: 6px; font-size: 12px; color: var(--text-secondary); cursor: pointer; flex: 1;">
							<input type="checkbox" bind:checked={cronEnabled} style="accent-color: var(--accent-indigo);" />
							Enabled
						</label>
						<button onclick={() => { showCronForm = false; resetCronForm(); }}
							style="padding: 6px 12px; border: 1px solid var(--border-default); background: none; border-radius: 6px; color: var(--text-muted); font-size: 12px; cursor: pointer; font-family: var(--font-ui);">
							Cancel
						</button>
						<button onclick={saveCron} disabled={!cronCanSave}
							style="padding: 6px 16px; border: none; border-radius: 6px; font-size: 12px; font-weight: 700; cursor: pointer; font-family: var(--font-ui); transition: all 0.15s;
								background: {!cronCanSave ? 'var(--text-muted)' : 'var(--accent-indigo)'}; color: white;">
							{cronSaving ? 'Saving...' : editingCron ? 'Update' : 'Create'}
						</button>
					</div>
				</div>
			{/if}

			<!-- Cron list -->
			{#if automation.loadingCrons}
				<div style="padding: 24px; text-align: center; color: var(--text-muted); font-size: 13px;">Loading...</div>
			{:else if automation.crons.length === 0 && !showCronForm}
				<div style="padding: 24px; text-align: center; color: var(--text-muted); font-size: 13px; background: var(--bg-primary); border-radius: 10px; border: 1px dashed var(--border-default);">
					No recurring tasks yet. Click "+ New Recurring Task" to create one.
				</div>
			{:else}
				{#each automation.crons as cron (cron.id)}
					{@const tpl = (cron.task_template ?? {}) as Record<string, any>}
					{@const repoParentTail = typeof tpl.repo_parent === 'string'
						? (String(tpl.repo_parent).split(/[\\/]/).filter(Boolean).pop() || String(tpl.repo_parent))
						: ''}
					<div style="padding: 12px 14px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 8px; display: flex; flex-direction: column; gap: 6px; transition: all 0.15s;">
						<div style="display: flex; align-items: center; gap: 8px;">
							<span style="
								width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0;
								background: {cron.enabled ? '#3fb950' : '#6e7681'};
								box-shadow: {cron.enabled ? '0 0 6px rgba(63,185,80,0.4)' : 'none'};
							"></span>
							<span style="font-size: 13px; font-weight: 600; color: var(--text-primary); flex: 1;">{cron.name}</span>
							<span style="padding: 2px 8px; border-radius: 4px; font-size: 11px; font-family: var(--font-mono, monospace); background: rgba(88,166,255,0.08); color: var(--accent-blue);">
								{humanSchedule(cron.schedule)}
							</span>
						</div>
						<div style="display: flex; align-items: center; gap: 8px; font-size: 11px; color: var(--text-muted); flex-wrap: wrap;">
							{#if tpl.repo_parent}
								<span style="padding: 1px 6px; border-radius: 4px; background: rgba(168, 85, 247, 0.1); color: #c084fc; font-family: var(--font-mono);" title={String(tpl.repo_parent)}>
									📂 {repoParentTail}/*
								</span>
							{:else if tpl.project}
								<span style="padding: 1px 6px; border-radius: 4px; background: rgba(99, 102, 241, 0.1); color: var(--accent-indigo); font-family: var(--font-mono);">
									@{tpl.project}
								</span>
							{:else}
								<span style="padding: 1px 6px; border-radius: 4px; background: rgba(248, 81, 73, 0.1); color: var(--accent-red);">no repo</span>
							{/if}
							<span>Last: {formatDate(cron.last_run)}</span>
							<span>Next: {formatDate(cron.next_run)}</span>
							<span style="flex: 1;"></span>
							<button onclick={() => editCron(cron)}
								style="padding: 3px 8px; border: 1px solid var(--border-default); background: none; border-radius: 4px; color: var(--text-muted); font-size: 10px; cursor: pointer; font-family: var(--font-ui); transition: all 0.15s;">
								Edit
							</button>
							<button onclick={() => automation.toggleCron(cron.id)}
								style="padding: 3px 8px; border: 1px solid var(--border-default); background: none; border-radius: 4px; color: {cron.enabled ? '#f85149' : '#3fb950'}; font-size: 10px; cursor: pointer; font-family: var(--font-ui); transition: all 0.15s;">
								{cron.enabled ? 'Disable' : 'Enable'}
							</button>
						</div>
					</div>
				{/each}
			{/if}
		</div>

	<!-- ═══ WEBHOOKS ═══════════════════════════════════════════ -->
	{:else}
		<div style="display: flex; flex-direction: column; gap: 16px;">
			<div style="display: flex; align-items: center;">
				<div style="{sectionTitle} flex: 1; border: none; padding: 0;">Webhook Triggers</div>
				<button
					onclick={() => { showWebhookForm = true; webhookName = ''; webhookTaskTitle = ''; }}
					style="padding: 5px 12px; border: 1px solid rgba(99,102,241,0.3); background: rgba(99,102,241,0.08); color: var(--accent-indigo); border-radius: 6px; font-size: 12px; font-weight: 600; cursor: pointer; font-family: var(--font-ui); transition: all 0.15s;"
				>
					+ New Webhook
				</button>
			</div>

			<!-- How it works -->
			<div style="padding: 12px 14px; background: rgba(99,102,241,0.04); border: 1px solid rgba(99,102,241,0.12); border-radius: 8px; font-size: 12px; color: var(--text-secondary); line-height: 1.6;">
				<div style="font-weight: 600; color: var(--text-primary); margin-bottom: 4px;">How webhooks work</div>
				Each webhook gets a unique URL. When something POSTs to that URL, Sam creates a task from the template.
				No authentication needed (the trigger ID in the URL is the secret).
				<div style="margin-top: 8px;">
					<span style="font-weight: 500; color: var(--text-primary);">Use cases:</span> GitHub push events, deploy notifications, external cron services (cron-job.org), Zapier/Make, CI/CD pipelines.
				</div>
				<details style="margin-top: 10px; font-size: 11px; color: var(--text-muted);">
					<summary style="cursor: pointer; user-select: none; font-size: 12px; font-weight: 600; color: var(--text-primary);">JSON payload schema</summary>
					<div style="margin-top: 6px; padding: 8px 10px; background: var(--bg-primary); border-radius: 6px; border: 1px solid var(--border-subtle); overflow-x: auto;">
						<code style="font-size: 11px; font-family: var(--font-mono, monospace); color: var(--text-secondary); white-space: pre; display: block; line-height: 1.6;">{`{
  "repo_url":    "https://github.com/you/repo",
  "title":       "Overrides default task title",
  "description": "What Sam should do",
  "priority":    "low | medium | high | critical",
  "project":     "project-name (alternative to repo_url)"
}`}</code>
					</div>
					<div style="margin-top: 6px; line-height: 1.5;">
						<strong style="color: var(--text-primary);">repo_url</strong> (recommended) resolves the project automatically from the project registry. Sam matches the URL to a registered project and fills in repo_path, preview_url, and project name. Accepts <code style="font-family: var(--font-mono, monospace); font-size: 10px; color: var(--accent-blue);">.git</code> suffixes and trailing slashes. If neither <code style="font-family: var(--font-mono, monospace); font-size: 10px; color: var(--accent-blue);">repo_url</code> nor <code style="font-family: var(--font-mono, monospace); font-size: 10px; color: var(--accent-blue);">project</code> is provided, the task may fail or require manual confirmation. All other fields are optional and override template defaults. The full payload is saved as <code style="font-family: var(--font-mono, monospace); font-size: 10px; color: var(--accent-blue);">context</code> on the task for Sam to reference during execution. Note: <code style="font-family: var(--font-mono, monospace); font-size: 10px; color: var(--accent-blue);">task_type</code> (code/research) is set by the template and cannot be overridden via payload.
					</div>
				</details>
			</div>

			<!-- Webhook form -->
			{#if showWebhookForm}
				<div style="padding: 16px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 10px; display: flex; flex-direction: column; gap: 12px;">
					<div style="font-size: 13px; font-weight: 700; color: var(--text-primary);">New Webhook Trigger</div>

					<div style="display: flex; flex-direction: column; gap: 4px;">
						<span style="{labelStyle}">Webhook Name</span>
						<input bind:value={webhookName} placeholder="e.g. GitHub Push, Deploy Hook" style="{inputStyle}" />
					</div>

					<div style="display: flex; flex-direction: column; gap: 4px;">
						<span style="{labelStyle}">Default Task Title</span>
						<input bind:value={webhookTaskTitle} placeholder="Leave blank to use webhook name" style="{inputStyle}" />
						<span style="font-size: 10px; color: var(--text-muted);">Fallback title when the webhook payload does not include a title field.</span>
					</div>

					<div style="display: grid; grid-template-columns: 1fr 1fr; gap: 10px;">
						<div style="display: flex; flex-direction: column; gap: 4px;">
							<span style="{labelStyle}">Task Type</span>
							<div style="display: flex; gap: 4px;">
								<button onclick={() => webhookTaskType = 'code'}
									style="flex: 1; padding: 6px; border-radius: 6px; font-size: 12px; font-weight: 600; cursor: pointer; font-family: var(--font-ui); transition: all 0.15s;
										border: 1px solid {webhookTaskType === 'code' ? 'rgba(99,102,241,0.4)' : 'var(--border-default)'};
										background: {webhookTaskType === 'code' ? 'rgba(99,102,241,0.1)' : 'var(--bg-surface)'};
										color: {webhookTaskType === 'code' ? 'var(--accent-indigo)' : 'var(--text-muted)'};">
									Code
								</button>
								<button onclick={() => webhookTaskType = 'research'}
									style="flex: 1; padding: 6px; border-radius: 6px; font-size: 12px; font-weight: 600; cursor: pointer; font-family: var(--font-ui); transition: all 0.15s;
										border: 1px solid {webhookTaskType === 'research' ? 'rgba(99,102,241,0.4)' : 'var(--border-default)'};
										background: {webhookTaskType === 'research' ? 'rgba(99,102,241,0.1)' : 'var(--bg-surface)'};
										color: {webhookTaskType === 'research' ? 'var(--accent-indigo)' : 'var(--text-muted)'};">
									Research
								</button>
							</div>
						</div>
						<div style="display: flex; flex-direction: column; gap: 4px;">
							<span style="{labelStyle}">Priority</span>
							<select bind:value={webhookPriority}
								style="padding: 8px 10px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 6px; color: var(--text-primary); font-size: 13px; font-family: var(--font-ui); outline: none; cursor: pointer;">
								{#each priorities as p}
									<option value={p.value}>{p.label}</option>
								{/each}
							</select>
						</div>
					</div>

					<div style="display: flex; gap: 6px; justify-content: flex-end; padding-top: 4px; border-top: 1px solid var(--border-subtle);">
						<button onclick={() => showWebhookForm = false}
							style="padding: 6px 12px; border: 1px solid var(--border-default); background: none; border-radius: 6px; color: var(--text-muted); font-size: 12px; cursor: pointer; font-family: var(--font-ui);">
							Cancel
						</button>
						<button onclick={createWebhook} disabled={!webhookName.trim() || webhookSaving}
							style="padding: 6px 16px; border: none; border-radius: 6px; font-size: 12px; font-weight: 700; cursor: pointer; font-family: var(--font-ui); transition: all 0.15s;
								background: {!webhookName.trim() ? 'var(--text-muted)' : 'var(--accent-indigo)'}; color: white;">
							{webhookSaving ? 'Creating...' : 'Create Webhook'}
						</button>
					</div>
				</div>
			{/if}

			<!-- Webhook list -->
			{#if automation.loadingTriggers}
				<div style="padding: 24px; text-align: center; color: var(--text-muted); font-size: 13px;">Loading...</div>
			{:else if webhookTriggers.length === 0 && !showWebhookForm}
				<div style="padding: 24px; text-align: center; color: var(--text-muted); font-size: 13px; background: var(--bg-primary); border-radius: 10px; border: 1px dashed var(--border-default);">
					No webhooks yet. Click "+ New Webhook" to create one.
				</div>
			{:else}
				{#each webhookTriggers as trigger (trigger.id)}
					<div style="padding: 12px 14px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 8px; display: flex; flex-direction: column; gap: 8px;">
						<div style="display: flex; align-items: center; gap: 8px;">
							<span style="
								width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0;
								background: {trigger.enabled ? '#3fb950' : '#6e7681'};
								box-shadow: {trigger.enabled ? '0 0 6px rgba(63,185,80,0.4)' : 'none'};
							"></span>
							<span style="font-size: 13px; font-weight: 600; color: var(--text-primary); flex: 1;">{trigger.name}</span>
							<span style="padding: 2px 6px; background: rgba(210,153,34,0.1); color: #d29922; border-radius: 4px; font-size: 10px; font-weight: 600;">WH</span>
							<button onclick={() => automation.toggleTrigger(trigger.id)}
								style="padding: 3px 8px; border: 1px solid var(--border-default); background: none; border-radius: 4px; color: {trigger.enabled ? '#f85149' : '#3fb950'}; font-size: 10px; cursor: pointer; font-family: var(--font-ui);">
								{trigger.enabled ? 'Disable' : 'Enable'}
							</button>
						</div>

						<!-- URL with copy button -->
						<div style="display: flex; align-items: center; gap: 6px; padding: 6px 10px; background: var(--bg-surface); border-radius: 6px; border: 1px solid var(--border-subtle);">
							<code style="flex: 1; font-size: 11px; color: var(--accent-blue); word-break: break-all; font-family: var(--font-mono, monospace);">
								{webhookUrl(trigger.id)}
							</code>
							<button
								onclick={() => copyToClipboard(webhookUrl(trigger.id), `url-${trigger.id}`)}
								style="padding: 4px 8px; border: 1px solid var(--border-default); background: {copiedId === `url-${trigger.id}` ? 'rgba(63,185,80,0.1)' : 'none'}; border-radius: 4px; color: {copiedId === `url-${trigger.id}` ? '#3fb950' : 'var(--text-muted)'}; font-size: 10px; cursor: pointer; font-family: var(--font-ui); transition: all 0.15s; flex-shrink: 0;"
							>
								{copiedId === `url-${trigger.id}` ? 'Copied!' : 'Copy'}
							</button>
						</div>

						<!-- Quick curl example -->
						<details style="font-size: 11px; color: var(--text-muted);">
							<summary style="cursor: pointer; user-select: none; padding: 2px 0;">Test with curl</summary>
							<div style="margin-top: 6px; padding: 8px 10px; background: var(--bg-surface); border-radius: 6px; border: 1px solid var(--border-subtle); overflow-x: auto;">
								<code style="font-size: 10px; font-family: var(--font-mono, monospace); color: var(--text-secondary); white-space: pre; display: block; line-height: 1.5;">{`curl -X POST ${webhookUrl(trigger.id)} \\
  -H "Content-Type: application/json" \\
  -d '{
    "repo_url": "https://github.com/you/repo",
    "title": "Fix login bug",
    "description": "Users report 500 on /auth/login",
    "priority": "high"
  }'`}</code>
								<div style="margin-top: 4px; font-size: 10px; color: var(--text-muted);">bash/zsh syntax. <code style="font-family: var(--font-mono, monospace);">repo_url</code> routes the task to the correct project automatically.</div>
								<div style="margin-top: 6px; display: flex; gap: 6px; flex-wrap: wrap;">
									<button
										onclick={() => copyToClipboard(`curl -X POST ${webhookUrl(trigger.id)} -H "Content-Type: application/json" -d '{"repo_url": "https://github.com/you/repo", "title": "Fix login bug", "description": "Users report 500 on /auth/login", "priority": "high"}'`, `curl-${trigger.id}`)}
										style="padding: 3px 8px; border: 1px solid var(--border-default); background: {copiedId === `curl-${trigger.id}` ? 'rgba(63,185,80,0.1)' : 'none'}; border-radius: 4px; color: {copiedId === `curl-${trigger.id}` ? '#3fb950' : 'var(--text-muted)'}; font-size: 10px; cursor: pointer; font-family: var(--font-ui); transition: all 0.15s;"
									>
										{copiedId === `curl-${trigger.id}` ? 'Copied!' : 'Copy curl (with example)'}
									</button>
									<button
										onclick={() => copyToClipboard(`curl -X POST ${webhookUrl(trigger.id)} -H "Content-Type: application/json" -d '{}'`, `curl-min-${trigger.id}`)}
										style="padding: 3px 8px; border: 1px solid var(--border-default); background: {copiedId === `curl-min-${trigger.id}` ? 'rgba(63,185,80,0.1)' : 'none'}; border-radius: 4px; color: {copiedId === `curl-min-${trigger.id}` ? '#3fb950' : 'var(--text-muted)'}; font-size: 10px; cursor: pointer; font-family: var(--font-ui); transition: all 0.15s;"
									>
										{copiedId === `curl-min-${trigger.id}` ? 'Copied!' : 'Copy curl (empty body)'}
									</button>
								</div>
							</div>
						</details>

						<div style="font-size: 10px; color: var(--text-muted);">
							Last checked: {formatDate(trigger.last_checked)}
						</div>
					</div>
				{/each}
			{/if}
		</div>
	{/if}
</div>
