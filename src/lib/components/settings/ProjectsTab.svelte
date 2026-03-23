<script lang="ts">
	import { onMount } from 'svelte';
	import { getProjectStore } from '$lib/stores/projects.svelte';
	import { getTheme } from '$lib/stores/theme.svelte';
	import type { AeProject } from '$lib/types';

	const projectStore = getProjectStore();
	const theme = getTheme();

	let showForm = $state(false);
	let editingId = $state<string | null>(null);
	let confirmDeleteId = $state<string | null>(null);

	// Form fields
	let name = $state('');
	let repoUrl = $state('');
	let repoPath = $state('');
	let previewUrl = $state('');
	let client = $state('');
	let deployMethod = $state('');
	let devCommand = $state('');

	onMount(() => {
		projectStore.fetchProjects();
	});

	function resetForm() {
		name = '';
		repoUrl = '';
		repoPath = '';
		previewUrl = '';
		client = '';
		deployMethod = '';
		devCommand = '';
		editingId = null;
		showForm = false;
	}

	function startEdit(project: AeProject) {
		name = project.name;
		repoUrl = project.repo_url || '';
		repoPath = project.repo_path || '';
		previewUrl = project.preview_url || '';
		client = project.client || '';
		deployMethod = project.deploy_method || '';
		devCommand = project.dev_command || '';
		editingId = project.id;
		showForm = true;
	}

	async function handleSubmit() {
		if (!name.trim()) return;

		const data: Partial<AeProject> = {
			name: name.trim(),
			repo_url: repoUrl.trim() || null,
			repo_path: repoPath.trim() || null,
			preview_url: previewUrl.trim() || null,
			client: client.trim() || null,
			deploy_method: deployMethod.trim() || null,
			dev_command: devCommand.trim() || null,
		};

		if (editingId) {
			await projectStore.updateProject(editingId, data);
		} else {
			await projectStore.createProject(data);
		}

		resetForm();
	}

	async function handleDelete(id: string) {
		if (confirmDeleteId !== id) {
			confirmDeleteId = id;
			setTimeout(() => confirmDeleteId = null, 3000);
			return;
		}
		await projectStore.deleteProject(id);
		confirmDeleteId = null;
	}
</script>

<div style="display: flex; flex-direction: column; gap: 16px; height: 100%; overflow-y: auto;">
	<!-- Header -->
	<div style="display: flex; align-items: center; justify-content: space-between;">
		<div>
			<div style="font-size: 15px; font-weight: 600; color: {theme.c.textPrimary};">Projects</div>
			<div style="font-size: 11px; color: {theme.c.textMuted}; margin-top: 2px;">
				Sam uses this registry to know your repos. No hallucinating.
			</div>
		</div>
		{#if !showForm}
			<button
				style="
					padding: 6px 14px; border-radius: 8px; font-size: 12px; font-weight: 600;
					background: {theme.c.accentPrimary}; color: #fff; border: none;
					cursor: pointer; font-family: var(--font-ui); transition: all 0.15s;
				"
				onclick={() => { resetForm(); showForm = true; }}
			>
				+ Add Project
			</button>
		{/if}
	</div>

	<!-- Form -->
	{#if showForm}
		<div style="
			padding: 16px; border-radius: 10px;
			background: {theme.c.bgSurface}; border: 1px solid {theme.c.borderDefault};
			display: flex; flex-direction: column; gap: 10px;
		">
			<div style="font-size: 13px; font-weight: 600; color: {theme.c.textPrimary};">
				{editingId ? 'Edit Project' : 'New Project'}
			</div>

			<div style="display: grid; grid-template-columns: 1fr 1fr; gap: 8px;">
				<div>
					<div style="font-size: 10px; color: {theme.c.textMuted}; margin-bottom: 3px; text-transform: uppercase;">Name *</div>
					<input bind:value={name} placeholder="operly"
						style="width: 100%; padding: 6px 10px; background: {theme.c.bgPrimary}; border: 1px solid {theme.c.borderDefault}; border-radius: 6px; color: {theme.c.textPrimary}; font-size: 12px; font-family: var(--font-mono); outline: none;" />
				</div>
				<div>
					<div style="font-size: 10px; color: {theme.c.textMuted}; margin-bottom: 3px; text-transform: uppercase;">Client / Group</div>
					<input bind:value={client} placeholder="personal"
						style="width: 100%; padding: 6px 10px; background: {theme.c.bgPrimary}; border: 1px solid {theme.c.borderDefault}; border-radius: 6px; color: {theme.c.textPrimary}; font-size: 12px; font-family: var(--font-mono); outline: none;" />
				</div>
			</div>

			<div>
				<div style="font-size: 10px; color: {theme.c.textMuted}; margin-bottom: 3px; text-transform: uppercase;">Repo URL</div>
				<input bind:value={repoUrl} placeholder="https://github.com/user/repo"
					style="width: 100%; padding: 6px 10px; background: {theme.c.bgPrimary}; border: 1px solid {theme.c.borderDefault}; border-radius: 6px; color: {theme.c.textPrimary}; font-size: 12px; font-family: var(--font-mono); outline: none;" />
			</div>

			<div>
				<div style="font-size: 10px; color: {theme.c.textMuted}; margin-bottom: 3px; text-transform: uppercase;">Local Path (on worker machine)</div>
				<input bind:value={repoPath} placeholder="C:\\Projects\\operly"
					style="width: 100%; padding: 6px 10px; background: {theme.c.bgPrimary}; border: 1px solid {theme.c.borderDefault}; border-radius: 6px; color: {theme.c.textPrimary}; font-size: 12px; font-family: var(--font-mono); outline: none;" />
			</div>

			<div style="display: grid; grid-template-columns: 1fr 1fr; gap: 8px;">
				<div>
					<div style="font-size: 10px; color: {theme.c.textMuted}; margin-bottom: 3px; text-transform: uppercase;">Preview URL</div>
					<input bind:value={previewUrl} placeholder="http://localhost:3000"
						style="width: 100%; padding: 6px 10px; background: {theme.c.bgPrimary}; border: 1px solid {theme.c.borderDefault}; border-radius: 6px; color: {theme.c.textPrimary}; font-size: 12px; font-family: var(--font-mono); outline: none;" />
				</div>
				<div>
					<div style="font-size: 10px; color: {theme.c.textMuted}; margin-bottom: 3px; text-transform: uppercase;">Deploy Method</div>
					<input bind:value={deployMethod} placeholder="vercel, railway, etc"
						style="width: 100%; padding: 6px 10px; background: {theme.c.bgPrimary}; border: 1px solid {theme.c.borderDefault}; border-radius: 6px; color: {theme.c.textPrimary}; font-size: 12px; font-family: var(--font-mono); outline: none;" />
				</div>
			</div>

			<div>
				<div style="font-size: 10px; color: {theme.c.textMuted}; margin-bottom: 3px; text-transform: uppercase;">Dev Command (optional, auto-detects from package.json)</div>
				<input bind:value={devCommand} placeholder="npm run dev -- --port {'{port}'}"
					style="width: 100%; padding: 6px 10px; background: {theme.c.bgPrimary}; border: 1px solid {theme.c.borderDefault}; border-radius: 6px; color: {theme.c.textPrimary}; font-size: 12px; font-family: var(--font-mono); outline: none;" />
			</div>

			<div style="display: flex; gap: 8px; justify-content: flex-end; margin-top: 4px;">
				<button
					style="padding: 6px 14px; border-radius: 6px; font-size: 12px; background: none; border: 1px solid {theme.c.borderDefault}; color: {theme.c.textSecondary}; cursor: pointer; font-family: var(--font-ui);"
					onclick={resetForm}
				>Cancel</button>
				<button
					style="padding: 6px 14px; border-radius: 6px; font-size: 12px; background: {theme.c.accentPrimary}; color: #fff; border: none; cursor: pointer; font-family: var(--font-ui); font-weight: 600;"
					onclick={handleSubmit}
				>{editingId ? 'Save' : 'Add'}</button>
			</div>
		</div>
	{/if}

	<!-- Project list -->
	{#if projectStore.loading}
		<div style="color: {theme.c.textMuted}; font-size: 12px; padding: 20px 0; text-align: center;">Loading...</div>
	{:else if projectStore.projects.length === 0}
		<div style="color: {theme.c.textMuted}; font-size: 12px; padding: 40px 0; text-align: center;">
			No projects yet. Add one so Sam knows where your code lives.
		</div>
	{:else}
		<div style="display: flex; flex-direction: column; gap: 6px;">
			{#each projectStore.projects as project (project.id)}
				<div style="
					padding: 10px 14px; border-radius: 8px;
					background: {theme.c.bgSurface}; border: 1px solid {theme.c.borderSubtle};
					display: flex; align-items: flex-start; gap: 12px;
					transition: border-color 0.15s;
				">
					<div style="flex: 1; min-width: 0;">
						<div style="display: flex; align-items: center; gap: 6px; margin-bottom: 3px;">
							<span style="font-size: 13px; font-weight: 600; color: {theme.c.textPrimary};">{project.name}</span>
							{#if project.client}
								<span style="font-size: 9px; padding: 1px 6px; border-radius: 4px; background: {theme.c.accentGlow}; color: {theme.c.accentIndigo}; font-weight: 600;">
									{project.client}
								</span>
							{/if}
						</div>
						{#if project.repo_url}
							<div style="font-size: 11px; color: {theme.c.accentBlue}; font-family: var(--font-mono); white-space: nowrap; overflow: hidden; text-overflow: ellipsis;">
								{project.repo_url}
							</div>
						{/if}
						{#if project.repo_path}
							<div style="font-size: 10px; color: {theme.c.textMuted}; font-family: var(--font-mono); margin-top: 1px;">
								{project.repo_path}
							</div>
						{/if}
					</div>

					<div style="display: flex; gap: 4px; flex-shrink: 0;">
						<button
							style="padding: 4px 8px; border-radius: 4px; font-size: 10px; background: none; border: 1px solid {theme.c.borderDefault}; color: {theme.c.textMuted}; cursor: pointer; font-family: var(--font-ui);"
							onclick={() => startEdit(project)}
						>Edit</button>
						<button
							style="padding: 4px 8px; border-radius: 4px; font-size: 10px; background: none; border: 1px solid {confirmDeleteId === project.id ? theme.c.accentRed + '60' : theme.c.borderDefault}; color: {confirmDeleteId === project.id ? theme.c.accentRed : theme.c.textMuted}; cursor: pointer; font-family: var(--font-ui);"
							onclick={() => handleDelete(project.id)}
						>{confirmDeleteId === project.id ? 'Confirm' : 'Delete'}</button>
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>
