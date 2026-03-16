<script lang="ts">
	import { getWorkspace } from '$lib/stores/workspace.svelte';
	import { getSettingsStore, updateSetting } from '$lib/stores/settings.svelte';
	import CloneRepoModal from '$lib/components/modals/CloneRepoModal.svelte';

	const workspace = getWorkspace();
	const settingsStore = getSettingsStore();

	let minimizeHovered = $state(false);
	let maximizeHovered = $state(false);
	let closeHovered = $state(false);
	let newProjectOpen = $state(false);
	let newProjectHovered = $state(false);
	let themeDropdownOpen = $state(false);
	let cloneModalOpen = $state(false);

	const THEMES = [
		{ id: 'banana', label: 'Banana', color: '#FFD60A' },
		{ id: 'blue', label: 'Blue', color: '#58A6FF' },
		{ id: 'green', label: 'Green', color: '#3FB950' },
		{ id: 'purple', label: 'Purple', color: '#BC8CFF' },
		{ id: 'red', label: 'Red', color: '#F85149' },
		{ id: 'orange', label: 'Orange', color: '#D29922' },
	];

	let currentTheme = $derived((() => {
		const path = workspace.path;
		if (!path) return 'banana';
		return settingsStore.value.workspaceThemes?.[path] || 'banana';
	})());

	let currentThemeColor = $derived(THEMES.find(t => t.id === currentTheme)?.color || '#FFD60A');

	function applyTheme(themeId: string) {
		if (themeId === 'banana') {
			document.documentElement.removeAttribute('data-theme');
		} else {
			document.documentElement.setAttribute('data-theme', themeId);
		}

		// Save to settings
		if (workspace.path) {
			const themes = { ...settingsStore.value.workspaceThemes, [workspace.path]: themeId };
			updateSetting('workspaceThemes', themes);
		}
		themeDropdownOpen = false;
	}

	async function handleOpenFolder() {
		newProjectOpen = false;
		try {
			const { invoke } = await import('@tauri-apps/api/core');
			await invoke('open_folder_in_new_window');
		} catch {
			// Fallback for browser dev
		}
	}

	function handleCloneRepo() {
		newProjectOpen = false;
		cloneModalOpen = true;
	}

	// Close dropdowns on outside click
	$effect(() => {
		if (!newProjectOpen && !themeDropdownOpen) return;
		function onClick() { newProjectOpen = false; themeDropdownOpen = false; }
		setTimeout(() => document.addEventListener('click', onClick), 0);
		return () => document.removeEventListener('click', onClick);
	});

	async function minimize() {
		try {
			const { getCurrentWindow } = await import('@tauri-apps/api/window');
			await getCurrentWindow().minimize();
		} catch { /* browser dev mode */ }
	}

	async function maximize() {
		try {
			const { getCurrentWindow } = await import('@tauri-apps/api/window');
			await getCurrentWindow().toggleMaximize();
		} catch { /* browser dev mode */ }
	}

	async function close() {
		try {
			const { getCurrentWindow } = await import('@tauri-apps/api/window');
			await getCurrentWindow().close();
		} catch { /* browser dev mode */ }
	}
</script>

<div class="titlebar" data-tauri-drag-region style="display: flex; align-items: center; height: 40px; padding: 0 14px; background: linear-gradient(180deg, #181E28 0%, #12171F 100%); box-shadow: 0 2px 8px rgba(0, 0, 0, 0.4), inset 0 1px 0 rgba(255, 255, 255, 0.04); gap: 0; position: relative; z-index: 5; border-bottom: 1px solid rgba(255, 214, 10, 0.06);">
	<!-- Left: Logo + Brand -->
	<div style="display: flex; align-items: center; gap: 8px; min-width: 180px;">
		<span class="banana-logo" style="font-size: 22px; animation: bob 3s ease-in-out infinite; filter: drop-shadow(0 0 10px rgba(255, 214, 10, 0.4));">🍌</span>
		<span style="font-family: var(--font-ui); font-weight: 700; font-size: 15px; color: var(--banana-yellow); letter-spacing: -0.3px; text-shadow: 0 0 20px rgba(255, 214, 10, 0.25);">Banana Code</span>
		<span style="font-size: 9px; color: var(--text-muted); font-family: var(--font-mono); background: rgba(255, 214, 10, 0.06); padding: 1px 6px; border-radius: 6px;">v0.1</span>

		<!-- New Project button -->
		<div style="position: relative; margin-left: 8px;">
			<button
				style="
					width: 24px; height: 24px; display: flex; align-items: center; justify-content: center;
					border: 1px solid {newProjectHovered ? 'var(--accent-primary)' : 'var(--border-default)'};
					border-radius: 6px; cursor: pointer; transition: all 0.15s ease;
					background: {newProjectHovered ? 'color-mix(in srgb, var(--accent-primary) 12%, transparent)' : 'transparent'};
					color: {newProjectHovered ? 'var(--accent-primary)' : 'var(--text-muted)'};
				"
				onmouseenter={() => newProjectHovered = true}
				onmouseleave={() => newProjectHovered = false}
				onclick={(e) => { e.stopPropagation(); newProjectOpen = !newProjectOpen; }}
				title="New Project Window"
			>
				<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
					<line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/>
				</svg>
			</button>

			{#if newProjectOpen}
				<div style="
					position: absolute; top: 100%; left: 0; margin-top: 6px; z-index: 100;
					background: var(--bg-elevated); border: 1px solid var(--border-default);
					border-radius: 10px; box-shadow: 0 8px 24px rgba(0,0,0,0.5);
					padding: 4px; min-width: 180px; overflow: hidden;
				">
					<button
						style="width: 100%; padding: 8px 12px; border: none; background: none; cursor: pointer; text-align: left; font-size: 12px; font-family: var(--font-ui); color: var(--text-secondary); border-radius: 6px; transition: background 0.1s; display: flex; align-items: center; gap: 8px;"
						onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = 'rgba(255,255,255,0.06)'; }}
						onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background = 'none'; }}
						onclick={handleOpenFolder}
					>
						<svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor"><path d="M1.75 1A1.75 1.75 0 0 0 0 2.75v10.5C0 14.216.784 15 1.75 15h12.5A1.75 1.75 0 0 0 16 13.25v-8.5A1.75 1.75 0 0 0 14.25 3H7.5a.25.25 0 0 1-.2-.1l-.9-1.2C6.07 1.26 5.55 1 5 1H1.75z"/></svg>
						Open Folder
					</button>
					<button
						style="width: 100%; padding: 8px 12px; border: none; background: none; cursor: pointer; text-align: left; font-size: 12px; font-family: var(--font-ui); color: var(--text-secondary); border-radius: 6px; transition: background 0.1s; display: flex; align-items: center; gap: 8px;"
						onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = 'rgba(255,255,255,0.06)'; }}
						onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background = 'none'; }}
						onclick={handleCloneRepo}
					>
						<svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor"><path d="M2 2.5A2.5 2.5 0 0 1 4.5 0h8.75a.75.75 0 0 1 .75.75v12.5a.75.75 0 0 1-.75.75h-2.5a.75.75 0 0 1 0-1.5h1.75v-2h-8a1 1 0 0 0-.714 1.7.75.75 0 1 1-1.072 1.05A2.495 2.495 0 0 1 2 11.5Zm10.5-1h-8a1 1 0 0 0-1 1v6.708A2.486 2.486 0 0 1 4.5 9h8ZM5 12.25a.25.25 0 0 1 .25-.25h3.5a.25.25 0 0 1 .25.25v3.25a.25.25 0 0 1-.4.2l-1.45-1.087a.25.25 0 0 0-.3 0L5.4 15.7a.25.25 0 0 1-.4-.2Z"/></svg>
						Clone Repository
					</button>
				</div>
			{/if}
		</div>
	</div>

	<!-- Center: Workspace name + Theme selector -->
	<div data-tauri-drag-region style="flex: 1; display: flex; align-items: center; justify-content: center; gap: 8px; font-size: 12px; color: var(--text-secondary); overflow: hidden;">
		<span data-tauri-drag-region style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">{workspace.name}</span>

		<!-- Theme color dot -->
		{#if workspace.isOpen}
			<div style="position: relative;">
				<button
					style="
						width: 14px; height: 14px; border-radius: 50%;
						background: {currentThemeColor};
						border: 2px solid rgba(255,255,255,0.15);
						cursor: pointer; transition: all 0.15s ease;
						box-shadow: 0 0 8px {currentThemeColor}40;
					"
					onclick={(e) => { e.stopPropagation(); themeDropdownOpen = !themeDropdownOpen; }}
					title="Change window theme"
				></button>

				{#if themeDropdownOpen}
					<div style="
						position: absolute; top: 100%; left: 50%; transform: translateX(-50%); margin-top: 8px; z-index: 100;
						background: var(--bg-elevated); border: 1px solid var(--border-default);
						border-radius: 10px; box-shadow: 0 8px 24px rgba(0,0,0,0.5);
						padding: 8px; display: flex; gap: 6px;
					">
						{#each THEMES as theme}
							<button
								style="
									width: 22px; height: 22px; border-radius: 50%;
									background: {theme.color};
									border: 2px solid {currentTheme === theme.id ? 'white' : 'rgba(255,255,255,0.1)'};
									cursor: pointer; transition: all 0.15s ease;
									transform: {currentTheme === theme.id ? 'scale(1.15)' : 'scale(1)'};
									box-shadow: {currentTheme === theme.id ? '0 0 12px ' + theme.color + '60' : 'none'};
								"
								onclick={() => applyTheme(theme.id)}
								title={theme.label}
							></button>
						{/each}
					</div>
				{/if}
			</div>
		{/if}
	</div>

	<!-- Right: Window controls -->
	<div style="display: flex; align-items: center; gap: 2px; min-width: 100px; justify-content: flex-end;">
		<button
			class="win-btn"
			style="width: 32px; height: 26px; display: flex; align-items: center; justify-content: center; border: none; background: {minimizeHovered ? 'var(--bg-elevated)' : 'transparent'}; color: var(--text-secondary); border-radius: 6px; cursor: pointer; font-size: 16px; transition: all 0.15s ease;"
			onmouseenter={() => minimizeHovered = true}
			onmouseleave={() => minimizeHovered = false}
			onclick={minimize}
			aria-label="Minimize"
		>
			<svg width="12" height="1" viewBox="0 0 12 1" fill="currentColor"><rect width="12" height="1" rx="0.5"/></svg>
		</button>
		<button
			class="win-btn"
			style="width: 32px; height: 26px; display: flex; align-items: center; justify-content: center; border: none; background: {maximizeHovered ? 'var(--bg-elevated)' : 'transparent'}; color: var(--text-secondary); border-radius: 6px; cursor: pointer; font-size: 16px; transition: all 0.15s ease;"
			onmouseenter={() => maximizeHovered = true}
			onmouseleave={() => maximizeHovered = false}
			onclick={maximize}
			aria-label="Maximize"
		>
			<svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1"><rect x="0.5" y="0.5" width="9" height="9" rx="1"/></svg>
		</button>
		<button
			class="win-btn"
			style="width: 32px; height: 26px; display: flex; align-items: center; justify-content: center; border: none; background: {closeHovered ? 'var(--accent-red)' : 'transparent'}; color: {closeHovered ? '#fff' : 'var(--text-secondary)'}; border-radius: 6px; cursor: pointer; font-size: 16px; transition: all 0.15s ease;"
			onmouseenter={() => closeHovered = true}
			onmouseleave={() => closeHovered = false}
			onclick={close}
			aria-label="Close"
		>
			<svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.3"><line x1="1" y1="1" x2="9" y2="9"/><line x1="9" y1="1" x2="1" y2="9"/></svg>
		</button>
	</div>
</div>

{#if cloneModalOpen}
	<CloneRepoModal onClose={() => cloneModalOpen = false} />
{/if}
