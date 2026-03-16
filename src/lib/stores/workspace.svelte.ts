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

		/** Set workspace path. */
		async setWorkspace(folderPath: string): Promise<void> {
			workspacePath = folderPath;
			lastWorkspacePath = folderPath;
		},
	};
}
