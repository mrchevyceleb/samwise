/** Workspace store using Svelte 5 runes */

let workspacePath = $state<string | null>(null);
let lastWorkspacePath = $state<string | null>(null);

export function getWorkspace() {
	return {
		get path() { return workspacePath; },
		set path(v: string | null) {
			workspacePath = v;
			if (v) lastWorkspacePath = v;
		},

		get lastPath() { return lastWorkspacePath; },

		get name(): string {
			if (!workspacePath) return 'No Project Open';
			const parts = workspacePath.replace(/\\/g, '/').split('/');
			return parts[parts.length - 1] || 'Untitled';
		},

		get isOpen(): boolean {
			return workspacePath !== null;
		},

		/** Open a folder using Tauri file dialog, then initialize everything. */
		async openFolder(): Promise<string | null> {
			try {
				const { open } = await import('@tauri-apps/plugin-dialog');
				const selected = await open({ directory: true, multiple: false, title: 'Open Folder' });
				if (selected && typeof selected === 'string') {
					await this.setWorkspace(selected);
					return selected;
				}
			} catch (e) {
				console.warn('[workspace] Dialog not available:', e);
			}
			return null;
		},

		/** Set workspace path and initialize file tree, git, and preview. */
		async setWorkspace(folderPath: string): Promise<void> {
			workspacePath = folderPath;
			lastWorkspacePath = folderPath;

			// Load file tree
			try {
				const { getFileTreeStore } = await import('$lib/stores/file-tree');
				const fileTree = getFileTreeStore();
				await fileTree.loadTree(folderPath);
			} catch (e) {
				console.warn('[workspace] Failed to load file tree:', e);
			}

			// Initialize git status (non-blocking)
			try {
				const { getGitStore } = await import('$lib/stores/git');
				const git = getGitStore();
				git.refresh(folderPath).catch((e: unknown) => {
					console.warn('[workspace] Git refresh failed (may not be a repo):', e);
				});
			} catch (e) {
				console.warn('[workspace] Git init failed:', e);
			}

			// Preview is handled reactively by PreviewPanel watching workspace.path
			// No additional action needed here.
		},
	};
}
