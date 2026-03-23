<script lang="ts">
	import { onMount } from 'svelte';
	import { getProjectStore } from '$lib/stores/projects.svelte';
	import { getTheme } from '$lib/stores/theme.svelte';
	import { getSettings, updateSetting } from '$lib/stores/settings.svelte';
	import { safeInvoke } from '$lib/utils/tauri';
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

	// Scan state
	interface DiscoveredRepo {
		name: string;
		path: string;
		remote_url: string | null;
	}

	let scanResults = $state<DiscoveredRepo[]>([]);
	let selectedRepos = $state<Set<string>>(new Set());
	let scanning = $state(false);
	let addingRepos = $state(false);
	let scanExpanded = $state(false);
	let removeFolderConfirm = $state<string | null>(null);
	let newRepoCount = $derived([...selectedRepos].filter(p => !registeredPaths.has(p.replace(/\\/g, '/').toLowerCase())).length);

	// Derive scan folders from settings
	let scanFolders = $derived(getSettings().scanFolders || []);

	// Derive which paths are already registered
	let registeredPaths = $derived(
		new Set(projectStore.projects.map(p => p.repo_path?.replace(/\\/g, '/').toLowerCase()).filter(Boolean))
	);

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

	// ── Scan folder management ──

	async function addScanFolder() {
		try {
			const { open } = await import('@tauri-apps/plugin-dialog');
			const selected = await open({ directory: true, multiple: false, title: 'Select folder to scan for repos' });
			if (selected && typeof selected === 'string') {
				const current = getSettings().scanFolders || [];
				const normalized = selected.replace(/\\/g, '/');
				if (!current.some(f => f.replace(/\\/g, '/') === normalized)) {
					updateSetting('scanFolders', [...current, selected]);
				}
			}
		} catch (e) {
			console.warn('[projects] Dialog not available:', e);
		}
	}

	function removeScanFolder(folder: string) {
		if (removeFolderConfirm !== folder) {
			removeFolderConfirm = folder;
			setTimeout(() => removeFolderConfirm = null, 3000);
			return;
		}
		const current = getSettings().scanFolders || [];
		updateSetting('scanFolders', current.filter(f => f !== folder));
		removeFolderConfirm = null;
	}

	let scanErrors = $state<string[]>([]);

	async function scanAllFolders() {
		scanning = true;
		scanResults = [];
		scanErrors = [];
		selectedRepos = new Set();

		try {
			const allRepos: DiscoveredRepo[] = [];
			for (const folder of scanFolders) {
				const repos = await safeInvoke<DiscoveredRepo[]>('scan_for_repos', { root: folder });
				if (repos) {
					allRepos.push(...repos);
				} else {
					scanErrors = [...scanErrors, `Could not scan: ${folder}`];
				}
			}
			scanResults = allRepos;

			// Auto-select repos that aren't already registered
			const autoSelected = new Set<string>();
			for (const repo of allRepos) {
				const normalizedPath = repo.path.replace(/\\/g, '/').toLowerCase();
				if (!registeredPaths.has(normalizedPath)) {
					autoSelected.add(repo.path);
				}
			}
			selectedRepos = autoSelected;
		} finally {
			scanning = false;
		}
	}

	function toggleRepo(path: string) {
		const next = new Set(selectedRepos);
		if (next.has(path)) {
			next.delete(path);
		} else {
			next.add(path);
		}
		selectedRepos = next;
	}

	function selectAllNew() {
		const next = new Set<string>();
		for (const repo of scanResults) {
			const normalizedPath = repo.path.replace(/\\/g, '/').toLowerCase();
			if (!registeredPaths.has(normalizedPath)) {
				next.add(repo.path);
			}
		}
		selectedRepos = next;
	}

	function deselectAll() {
		selectedRepos = new Set();
	}

	async function addSelectedRepos() {
		addingRepos = true;
		let added = 0;
		for (const repo of scanResults) {
			if (!selectedRepos.has(repo.path)) continue;
			// Derive a client name from the parent folder
			const parentFolder = repo.path.replace(/\\/g, '/').split('/').slice(-2, -1)[0] || null;
			await projectStore.createProject({
				name: repo.name,
				repo_url: repo.remote_url || null,
				repo_path: repo.path,
				client: parentFolder,
			});
			added++;
		}
		// Clear results after adding
		scanResults = [];
		selectedRepos = new Set();
		addingRepos = false;
	}

	// Helper: folder display name
	function folderDisplayName(path: string): string {
		const parts = path.replace(/\\/g, '/').split('/');
		return parts[parts.length - 1] || path;
	}

	function isAlreadyRegistered(repoPath: string): boolean {
		return registeredPaths.has(repoPath.replace(/\\/g, '/').toLowerCase());
	}
</script>

<div style="display: flex; flex-direction: column; gap: 16px; height: 100%; overflow-y: auto;">
	<!-- Scan Folders Section -->
	<div style="
		border-radius: 10px; flex-shrink: 0;
		background: {theme.c.bgSurface}; border: 1px solid {theme.c.borderDefault};
		overflow: hidden; transition: all 0.2s;
	">
		<!-- Scan header (always visible) -->
		<button
			onclick={() => scanExpanded = !scanExpanded}
			style="
				width: 100%; padding: 12px 16px;
				display: flex; align-items: center; gap: 10px;
				background: none; border: none; cursor: pointer;
				color: {theme.c.textPrimary}; font-family: var(--font-ui);
			"
		>
			<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke={theme.c.accentIndigo} stroke-width="2"
				style="transition: transform 0.2s; transform: rotate({scanExpanded ? '90deg' : '0deg'});">
				<path d="M9 18l6-6-6-6"/>
			</svg>
			<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke={theme.c.accentIndigo} stroke-width="1.5">
				<path d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"/>
			</svg>
			<span style="font-size: 13px; font-weight: 600; flex: 1; text-align: left;">Scan Folders</span>
			<span style="font-size: 10px; color: {theme.c.textMuted};">
				{scanFolders.length} folder{scanFolders.length !== 1 ? 's' : ''} watched
			</span>
		</button>

		{#if scanExpanded}
			<div style="padding: 0 16px 16px; display: flex; flex-direction: column; gap: 10px;">
				<div style="font-size: 11px; color: {theme.c.textMuted};">
					Add folders to auto-discover git repos inside them. Like Fork, but for Sam.
				</div>

				<!-- Watched folders list -->
				{#if scanFolders.length > 0}
					<div style="display: flex; flex-direction: column; gap: 4px;">
						{#each scanFolders as folder (folder)}
							<div style="
								display: flex; align-items: center; gap: 8px;
								padding: 6px 10px; border-radius: 6px;
								background: {theme.c.bgPrimary}; border: 1px solid {theme.c.borderSubtle};
							">
								<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke={theme.c.accentAmber} stroke-width="1.5">
									<path d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"/>
								</svg>
								<span style="flex: 1; font-size: 11px; font-family: var(--font-mono); color: {theme.c.textSecondary}; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
									{folder}
								</span>
								<button
									onclick={() => removeScanFolder(folder)}
									style="
										padding: 2px 6px; border-radius: 4px; font-size: 9px;
										background: none; cursor: pointer; font-family: var(--font-ui);
										border: 1px solid {removeFolderConfirm === folder ? theme.c.accentRed + '60' : theme.c.borderDefault};
										color: {removeFolderConfirm === folder ? theme.c.accentRed : theme.c.textMuted};
										transition: all 0.15s;
									"
								>{removeFolderConfirm === folder ? 'Confirm' : 'Remove'}</button>
							</div>
						{/each}
					</div>
				{/if}

				<!-- Action buttons -->
				<div style="display: flex; gap: 8px;">
					<button
						onclick={addScanFolder}
						style="
							padding: 6px 12px; border-radius: 6px; font-size: 11px; font-weight: 600;
							background: none; border: 1px dashed {theme.c.borderDefault};
							color: {theme.c.textSecondary}; cursor: pointer; font-family: var(--font-ui);
							transition: all 0.15s;
						"
					>+ Add Folder</button>
					{#if scanFolders.length > 0}
						<button
							onclick={scanAllFolders}
							disabled={scanning}
							style="
								padding: 6px 14px; border-radius: 6px; font-size: 11px; font-weight: 600;
								background: {theme.c.accentIndigo}; color: #fff; border: none;
								cursor: {scanning ? 'wait' : 'pointer'}; font-family: var(--font-ui);
								opacity: {scanning ? '0.7' : '1'}; transition: all 0.15s;
							"
						>{scanning ? 'Scanning...' : 'Scan Now'}</button>
					{/if}
				</div>

				<!-- Scan Errors -->
				{#if scanErrors.length > 0}
					<div style="display: flex; flex-direction: column; gap: 3px; margin-top: 4px;">
						{#each scanErrors as err}
							<div style="font-size: 11px; color: {theme.c.accentRed}; padding: 4px 8px; border-radius: 4px; background: {theme.c.accentRed}10;">
								{err}
							</div>
						{/each}
					</div>
				{/if}

				<!-- Scan Results -->
				{#if scanResults.length > 0}
					<div style="margin-top: 4px;">
						<div style="display: flex; align-items: center; justify-content: space-between; margin-bottom: 6px;">
							<span style="font-size: 12px; font-weight: 600; color: {theme.c.textPrimary};">
								Found {scanResults.length} repo{scanResults.length !== 1 ? 's' : ''}
							</span>
							<div style="display: flex; gap: 8px;">
								<button
									onclick={selectAllNew}
									style="font-size: 10px; color: {theme.c.accentBlue}; background: none; border: none; cursor: pointer; font-family: var(--font-ui); text-decoration: underline;"
								>Select new</button>
								<button
									onclick={deselectAll}
									style="font-size: 10px; color: {theme.c.textMuted}; background: none; border: none; cursor: pointer; font-family: var(--font-ui); text-decoration: underline;"
								>Deselect all</button>
							</div>
						</div>

						<div style="display: flex; flex-direction: column; gap: 3px; max-height: 240px; overflow-y: auto;">
							{#each scanResults as repo (repo.path)}
								{@const already = isAlreadyRegistered(repo.path)}
								{@const checked = selectedRepos.has(repo.path)}
								<button
									onclick={() => !already && toggleRepo(repo.path)}
									disabled={already}
									style="
										display: flex; align-items: center; gap: 8px;
										padding: 6px 10px; border-radius: 6px; text-align: left;
										background: {checked && !already ? theme.c.accentIndigo + '10' : theme.c.bgPrimary};
										border: 1px solid {checked && !already ? theme.c.accentIndigo + '40' : theme.c.borderSubtle};
										cursor: {already ? 'default' : 'pointer'};
										opacity: {already ? '0.5' : '1'};
										transition: all 0.12s; font-family: var(--font-ui);
									"
								>
									<!-- Checkbox -->
									<div style="
										width: 16px; height: 16px; border-radius: 4px; flex-shrink: 0;
										border: 1.5px solid {checked && !already ? theme.c.accentIndigo : theme.c.borderDefault};
										background: {checked && !already ? theme.c.accentIndigo : 'transparent'};
										display: flex; align-items: center; justify-content: center;
										transition: all 0.12s;
									">
										{#if checked && !already}
											<svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="#fff" stroke-width="3">
												<path d="M20 6L9 17l-5-5"/>
											</svg>
										{/if}
									</div>

									<div style="flex: 1; min-width: 0;">
										<div style="display: flex; align-items: center; gap: 6px;">
											<span style="font-size: 12px; font-weight: 600; color: {theme.c.textPrimary};">{repo.name}</span>
											{#if already}
												<span style="font-size: 9px; padding: 1px 5px; border-radius: 3px; background: {theme.c.accentGreen}20; color: {theme.c.accentGreen}; font-weight: 600;">
													registered
												</span>
											{/if}
										</div>
										{#if repo.remote_url}
											<div style="font-size: 10px; color: {theme.c.accentBlue}; font-family: var(--font-mono); white-space: nowrap; overflow: hidden; text-overflow: ellipsis;">
												{repo.remote_url}
											</div>
										{/if}
										<div style="font-size: 9px; color: {theme.c.textMuted}; font-family: var(--font-mono); margin-top: 1px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;">
											{repo.path}
										</div>
									</div>
								</button>
							{/each}
						</div>

						<!-- Add selected button -->
						{#if newRepoCount > 0}
							<button
								onclick={addSelectedRepos}
								disabled={addingRepos}
								style="
									margin-top: 8px; padding: 8px 16px; border-radius: 8px;
									font-size: 12px; font-weight: 600; width: 100%;
									background: {theme.c.accentGreen}; color: #fff; border: none;
									cursor: {addingRepos ? 'wait' : 'pointer'}; font-family: var(--font-ui);
									opacity: {addingRepos ? '0.7' : '1'}; transition: all 0.15s;
								"
							>{addingRepos ? 'Adding...' : `Add ${newRepoCount} project${newRepoCount !== 1 ? 's' : ''} to registry`}</button>
						{/if}
					</div>
				{/if}
			</div>
		{/if}
	</div>

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
