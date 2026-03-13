/** Workspace store using Svelte 5 runes */

let workspacePath = $state<string | null>(null);

export function getWorkspace() {
	return {
		get path() { return workspacePath; },
		set path(v: string | null) { workspacePath = v; },

		get name(): string {
			if (!workspacePath) return 'No Project Open';
			const parts = workspacePath.replace(/\\/g, '/').split('/');
			return parts[parts.length - 1] || 'Untitled';
		},

		get isOpen(): boolean {
			return workspacePath !== null;
		}
	};
}
