<script lang="ts">
	import { getFileTreeStore, type FileNode } from '$lib/stores/file-tree';
	import { getWorkspace } from '$lib/stores/workspace';
	import FileTree from './FileTree.svelte';

	const fileTree = getFileTreeStore();
	const workspace = getWorkspace();

	let activeTab = $state<'files' | 'git'>('files');
	let filesTabHovered = $state(false);
	let gitTabHovered = $state(false);
	let openFolderHovered = $state(false);
	let collapseHovered = $state(false);

	let hasFiles = $derived(fileTree.tree.length > 0);
	let fileCount = $derived(fileTree.fileCount);

	async function openFolder() {
		try {
			const { open } = await import('@tauri-apps/plugin-dialog');
			const selected = await open({ directory: true, multiple: false, title: 'Open Folder' });
			if (selected && typeof selected === 'string') {
				workspace.path = selected;
				await fileTree.loadTree(selected);
			}
		} catch {
			console.warn('Dialog not available in browser dev mode');
		}
	}

	function handleFileClick(node: FileNode) {
		fileTree.selectFile(node.path);
		// TODO: open in editor
		console.log('File clicked:', node.path);
	}

	function handleFileDoubleClick(node: FileNode) {
		// TODO: open in editor tab
		console.log('File double-clicked:', node.path);
	}

	function collapseAll() {
		fileTree.collapseAll();
	}

	// Auto-load tree if workspace is already set
	$effect(() => {
		if (workspace.path && fileTree.tree.length === 0 && !fileTree.isLoading) {
			fileTree.loadTree(workspace.path);
		}
	});
</script>

<div style="display: flex; flex-direction: column; height: 100%; background: var(--bg-surface); border-left: 1px solid var(--border-default);">
	<!-- Tabs -->
	<div style="display: flex; height: 36px; border-bottom: 1px solid var(--border-default); flex-shrink: 0;">
		<button
			style="
				flex: 1; display: flex; align-items: center; justify-content: center; gap: 6px;
				background: none; border: none;
				border-bottom: 2px solid {activeTab === 'files' ? 'var(--banana-yellow)' : 'transparent'};
				color: {activeTab === 'files' ? 'var(--text-primary)' : filesTabHovered ? 'var(--text-primary)' : 'var(--text-secondary)'};
				cursor: pointer; font-family: var(--font-ui); font-size: 12px;
				font-weight: {activeTab === 'files' ? '600' : '400'};
				transition: all 0.12s ease;
			"
			onclick={() => activeTab = 'files'}
			onmouseenter={() => filesTabHovered = true}
			onmouseleave={() => filesTabHovered = false}
		>
			<svg width="13" height="13" viewBox="0 0 16 16" fill="currentColor">
				<path d="M1.75 1A1.75 1.75 0 0 0 0 2.75v10.5C0 14.216.784 15 1.75 15h12.5A1.75 1.75 0 0 0 16 13.25v-8.5A1.75 1.75 0 0 0 14.25 3H7.5a.25.25 0 0 1-.2-.1l-.9-1.2C6.07 1.26 5.55 1 5 1H1.75z"/>
			</svg>
			Files
			{#if hasFiles}
				<span style="font-size: 9px; color: var(--text-muted); background: rgba(255,255,255,0.06); padding: 0 5px; border-radius: 6px;">
					{fileCount}
				</span>
			{/if}
		</button>
		<button
			style="
				flex: 1; display: flex; align-items: center; justify-content: center; gap: 6px;
				background: none; border: none;
				border-bottom: 2px solid {activeTab === 'git' ? 'var(--banana-yellow)' : 'transparent'};
				color: {activeTab === 'git' ? 'var(--text-primary)' : gitTabHovered ? 'var(--text-primary)' : 'var(--text-secondary)'};
				cursor: pointer; font-family: var(--font-ui); font-size: 12px;
				font-weight: {activeTab === 'git' ? '600' : '400'};
				transition: all 0.12s ease;
			"
			onclick={() => activeTab = 'git'}
			onmouseenter={() => gitTabHovered = true}
			onmouseleave={() => gitTabHovered = false}
		>
			<svg width="13" height="13" viewBox="0 0 16 16" fill="currentColor">
				<path d="M9.5 3.25a2.25 2.25 0 1 1 3 2.122V6A2.5 2.5 0 0 1 10 8.5H6a1 1 0 0 0-1 1v1.128a2.251 2.251 0 1 1-1.5 0V5.372a2.25 2.25 0 1 1 1.5 0v1.836A2.492 2.492 0 0 1 6 7h4a1 1 0 0 0 1-1v-.628A2.25 2.25 0 0 1 9.5 3.25z"/>
			</svg>
			Git
		</button>
	</div>

	<!-- Content area -->
	{#if activeTab === 'files'}
		{#if hasFiles}
			<!-- Toolbar -->
			<div style="display: flex; align-items: center; gap: 4px; padding: 4px 8px; border-bottom: 1px solid var(--border-default); flex-shrink: 0;">
				<span style="flex: 1; font-size: 10px; color: var(--text-muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
					{workspace.name}
				</span>
				<button
					style="
						display: flex; align-items: center; justify-content: center;
						width: 22px; height: 22px; border-radius: 4px;
						background: {collapseHovered ? 'rgba(255,255,255,0.06)' : 'transparent'};
						border: none; color: var(--text-muted); cursor: pointer;
						transition: all 0.1s ease;
					"
					onclick={collapseAll}
					onmouseenter={() => collapseHovered = true}
					onmouseleave={() => collapseHovered = false}
					title="Collapse All"
				>
					<svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
						<path d="M4 4l4 4 4-4M4 8l4 4 4-4" stroke="currentColor" stroke-width="1.5" fill="none"/>
					</svg>
				</button>
				<button
					style="
						display: flex; align-items: center; justify-content: center;
						width: 22px; height: 22px; border-radius: 4px;
						background: {openFolderHovered ? 'rgba(255,255,255,0.06)' : 'transparent'};
						border: none; color: var(--text-muted); cursor: pointer;
						transition: all 0.1s ease;
					"
					onclick={openFolder}
					onmouseenter={() => openFolderHovered = true}
					onmouseleave={() => openFolderHovered = false}
					title="Open Folder"
				>
					<svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
						<path d="M1.75 1A1.75 1.75 0 0 0 0 2.75v10.5C0 14.216.784 15 1.75 15h12.5A1.75 1.75 0 0 0 16 13.25v-8.5A1.75 1.75 0 0 0 14.25 3H7.5a.25.25 0 0 1-.2-.1l-.9-1.2C6.07 1.26 5.55 1 5 1H1.75z"/>
					</svg>
				</button>
			</div>

			<!-- File tree -->
			<FileTree
				onFileClick={handleFileClick}
				onFileDoubleClick={handleFileDoubleClick}
			/>
		{:else}
			<!-- Empty state -->
			<div style="flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; padding: 24px; gap: 12px; text-align: center;">
				<svg width="40" height="40" viewBox="0 0 16 16" fill="var(--text-muted)" style="opacity: 0.4;">
					<path d="M1.75 1A1.75 1.75 0 0 0 0 2.75v10.5C0 14.216.784 15 1.75 15h12.5A1.75 1.75 0 0 0 16 13.25v-8.5A1.75 1.75 0 0 0 14.25 3H7.5a.25.25 0 0 1-.2-.1l-.9-1.2C6.07 1.26 5.55 1 5 1H1.75z"/>
				</svg>
				<p style="font-size: 13px; color: var(--text-muted); max-width: 180px; line-height: 1.5;">
					Open a folder to browse files
				</p>
				<button
					style="
						display: flex; align-items: center; gap: 6px;
						padding: 6px 14px;
						background: {openFolderHovered ? 'var(--banana-yellow)' : 'rgba(255, 214, 10, 0.12)'};
						border: 1px solid var(--banana-yellow); border-radius: 8px;
						color: {openFolderHovered ? '#0D1117' : 'var(--banana-yellow)'};
						cursor: pointer; font-family: var(--font-ui);
						font-size: 12px; font-weight: 600;
						transition: all 0.15s ease;
						transform: {openFolderHovered ? 'scale(1.05)' : 'scale(1)'};
					"
					onmouseenter={() => openFolderHovered = true}
					onmouseleave={() => openFolderHovered = false}
					onclick={openFolder}
				>
					Open Folder
				</button>
			</div>
		{/if}
	{:else}
		<!-- Git tab - coming soon -->
		<div style="flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; padding: 24px; gap: 12px; text-align: center;">
			<svg width="40" height="40" viewBox="0 0 16 16" fill="var(--text-muted)" style="opacity: 0.4;">
				<path d="M9.5 3.25a2.25 2.25 0 1 1 3 2.122V6A2.5 2.5 0 0 1 10 8.5H6a1 1 0 0 0-1 1v1.128a2.251 2.251 0 1 1-1.5 0V5.372a2.25 2.25 0 1 1 1.5 0v1.836A2.492 2.492 0 0 1 6 7h4a1 1 0 0 0 1-1v-.628A2.25 2.25 0 0 1 9.5 3.25z"/>
			</svg>
			<p style="font-size: 14px; font-weight: 600; color: var(--text-primary); margin-bottom: 0;">
				Git Integration
			</p>
			<p style="font-size: 12px; color: var(--text-muted);">Coming soon</p>
		</div>
	{/if}
</div>
