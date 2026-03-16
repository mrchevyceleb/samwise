<script lang="ts">
	interface Props {
		onClose: () => void;
	}

	let { onClose }: Props = $props();

	let repoUrl = $state('');
	let targetDir = $state('');
	let isCloning = $state(false);
	let error = $state('');
	let cloneHovered = $state(false);
	let browseHovered = $state(false);
	let cancelHovered = $state(false);

	async function browseFolder() {
		try {
			const { invoke } = await import('@tauri-apps/api/core');
			// Use Tauri dialog to pick folder
			const { open } = await import('@tauri-apps/plugin-dialog');
			const selected = await open({
				directory: true,
				title: 'Select Target Directory',
			});
			if (selected) {
				targetDir = typeof selected === 'string' ? selected : String(selected);
				// Append repo name from URL
				const repoName = repoUrl.split('/').pop()?.replace('.git', '') || '';
				if (repoName && targetDir) {
					targetDir = targetDir.replace(/[\\/]$/, '') + '/' + repoName;
				}
			}
		} catch (e) {
			console.warn('Browse failed:', e);
		}
	}

	async function handleClone() {
		if (!repoUrl.trim() || !targetDir.trim()) return;

		isCloning = true;
		error = '';

		try {
			const { invoke } = await import('@tauri-apps/api/core');
			await invoke('git_clone_repo', { url: repoUrl.trim(), targetDir: targetDir.trim() });
			await invoke('open_path_in_new_window', { path: targetDir.trim() });
			onClose();
		} catch (e) {
			error = String(e);
		} finally {
			isCloning = false;
		}
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') onClose();
		if (e.key === 'Enter' && !isCloning) handleClone();
	}
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	style="
		position: fixed; inset: 0; z-index: 200;
		background: rgba(0,0,0,0.6); backdrop-filter: blur(4px);
		display: flex; align-items: center; justify-content: center;
	"
	onclick={onClose}
>
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		style="
			width: 480px; background: var(--bg-elevated); border: 1px solid var(--border-default);
			border-radius: 14px; box-shadow: 0 16px 48px rgba(0,0,0,0.5);
			padding: 24px; display: flex; flex-direction: column; gap: 16px;
		"
		onclick={(e) => e.stopPropagation()}
	>
		<!-- Header -->
		<div style="display: flex; align-items: center; gap: 10px;">
			<svg width="20" height="20" viewBox="0 0 16 16" fill="var(--accent-primary)">
				<path d="M2 2.5A2.5 2.5 0 0 1 4.5 0h8.75a.75.75 0 0 1 .75.75v12.5a.75.75 0 0 1-.75.75h-2.5a.75.75 0 0 1 0-1.5h1.75v-2h-8a1 1 0 0 0-.714 1.7.75.75 0 1 1-1.072 1.05A2.495 2.495 0 0 1 2 11.5Zm10.5-1h-8a1 1 0 0 0-1 1v6.708A2.486 2.486 0 0 1 4.5 9h8ZM5 12.25a.25.25 0 0 1 .25-.25h3.5a.25.25 0 0 1 .25.25v3.25a.25.25 0 0 1-.4.2l-1.45-1.087a.25.25 0 0 0-.3 0L5.4 15.7a.25.25 0 0 1-.4-.2Z"/>
			</svg>
			<span style="font-size: 16px; font-weight: 700; color: var(--text-primary); font-family: var(--font-ui);">Clone Repository</span>
		</div>

		<!-- Repo URL -->
		<div style="display: flex; flex-direction: column; gap: 6px;">
			<label style="font-size: 12px; font-weight: 600; color: var(--text-secondary); font-family: var(--font-ui);">Repository URL</label>
			<input
				type="text"
				bind:value={repoUrl}
				placeholder="https://github.com/user/repo.git"
				style="
					width: 100%; padding: 10px 12px; background: var(--bg-primary);
					border: 1px solid var(--border-default); border-radius: 8px;
					color: var(--text-primary); font-family: var(--font-mono); font-size: 13px;
					outline: none; transition: border-color 0.15s ease;
				"
				onfocus={(e) => { (e.currentTarget as HTMLElement).style.borderColor = 'var(--accent-primary)'; }}
				onblur={(e) => { (e.currentTarget as HTMLElement).style.borderColor = 'var(--border-default)'; }}
			/>
		</div>

		<!-- Target directory -->
		<div style="display: flex; flex-direction: column; gap: 6px;">
			<label style="font-size: 12px; font-weight: 600; color: var(--text-secondary); font-family: var(--font-ui);">Target Directory</label>
			<div style="display: flex; gap: 8px;">
				<input
					type="text"
					bind:value={targetDir}
					placeholder="C:/Projects/my-repo"
					style="
						flex: 1; padding: 10px 12px; background: var(--bg-primary);
						border: 1px solid var(--border-default); border-radius: 8px;
						color: var(--text-primary); font-family: var(--font-mono); font-size: 13px;
						outline: none; transition: border-color 0.15s ease;
					"
					onfocus={(e) => { (e.currentTarget as HTMLElement).style.borderColor = 'var(--accent-primary)'; }}
					onblur={(e) => { (e.currentTarget as HTMLElement).style.borderColor = 'var(--border-default)'; }}
				/>
				<button
					style="
						padding: 8px 14px; border: 1px solid var(--border-default);
						border-radius: 8px; cursor: pointer; font-family: var(--font-ui);
						font-size: 12px; font-weight: 600; transition: all 0.15s ease;
						background: {browseHovered ? 'rgba(255,255,255,0.06)' : 'var(--bg-surface)'};
						color: var(--text-secondary);
					"
					onmouseenter={() => browseHovered = true}
					onmouseleave={() => browseHovered = false}
					onclick={browseFolder}
				>
					Browse
				</button>
			</div>
		</div>

		<!-- Error -->
		{#if error}
			<div style="padding: 8px 12px; background: rgba(248, 81, 73, 0.1); border: 1px solid rgba(248, 81, 73, 0.2); border-radius: 8px; font-size: 12px; color: var(--accent-red); font-family: var(--font-mono);">
				{error}
			</div>
		{/if}

		<!-- Actions -->
		<div style="display: flex; justify-content: flex-end; gap: 8px; margin-top: 4px;">
			<button
				style="
					padding: 8px 18px; border: 1px solid var(--border-default);
					border-radius: 8px; cursor: pointer; font-family: var(--font-ui);
					font-size: 12px; font-weight: 600; transition: all 0.15s ease;
					background: {cancelHovered ? 'rgba(255,255,255,0.06)' : 'transparent'};
					color: var(--text-secondary);
				"
				onmouseenter={() => cancelHovered = true}
				onmouseleave={() => cancelHovered = false}
				onclick={onClose}
			>
				Cancel
			</button>
			<button
				style="
					padding: 8px 24px; border: none; border-radius: 8px;
					cursor: {isCloning ? 'wait' : 'pointer'}; font-family: var(--font-ui);
					font-size: 12px; font-weight: 700; transition: all 0.15s ease;
					background: {isCloning ? 'var(--accent-dim)' : cloneHovered ? 'var(--accent-hover)' : 'var(--accent-primary)'};
					color: #0D1117;
					transform: {cloneHovered && !isCloning ? 'translateY(-1px)' : 'translateY(0)'};
					box-shadow: {cloneHovered && !isCloning ? '0 4px 16px color-mix(in srgb, var(--accent-primary) 35%, transparent)' : '0 2px 8px rgba(0,0,0,0.2)'};
					opacity: {(!repoUrl.trim() || !targetDir.trim()) ? '0.5' : '1'};
				"
				onmouseenter={() => cloneHovered = true}
				onmouseleave={() => cloneHovered = false}
				onclick={handleClone}
				disabled={isCloning || !repoUrl.trim() || !targetDir.trim()}
			>
				{isCloning ? 'Cloning...' : 'Clone & Open'}
			</button>
		</div>
	</div>
</div>
