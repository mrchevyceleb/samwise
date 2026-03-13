/** File tree store using Svelte 5 runes */

import { readDirectoryTree, type FileNode } from '$lib/utils/tauri';

export type { FileNode };

let tree = $state<FileNode[]>([]);
let expandedDirs = $state<Set<string>>(new Set());
let selectedPath = $state<string | null>(null);
let isLoading = $state(false);
let rootPath = $state<string | null>(null);

export function getFileTreeStore() {
	return {
		get tree() { return tree; },
		get expandedDirs() { return expandedDirs; },
		get selectedPath() { return selectedPath; },
		set selectedPath(p: string | null) { selectedPath = p; },
		get isLoading() { return isLoading; },
		get rootPath() { return rootPath; },

		get fileCount(): number {
			function count(nodes: FileNode[]): number {
				let c = 0;
				for (const n of nodes) {
					if (!n.is_dir) c++;
					if (n.children) c += count(n.children);
				}
				return c;
			}
			return count(tree);
		},

		async loadTree(path: string): Promise<void> {
			isLoading = true;
			rootPath = path;
			try {
				const rootNode = await readDirectoryTree(path);
				// The backend returns the root directory as a single FileNode; use its children as tree roots
				tree = rootNode.children ?? [];
				// Auto-expand root level
				for (const node of tree) {
					if (node.is_dir) {
						expandedDirs = new Set([...expandedDirs, node.path]);
					}
				}
			} catch (e) {
				console.error('Failed to load file tree:', e);
				tree = [];
			} finally {
				isLoading = false;
			}
		},

		toggleDir(path: string): void {
			const next = new Set(expandedDirs);
			if (next.has(path)) {
				next.delete(path);
			} else {
				next.add(path);
			}
			expandedDirs = next;
		},

		expandDir(path: string): void {
			if (!expandedDirs.has(path)) {
				expandedDirs = new Set([...expandedDirs, path]);
			}
		},

		collapseDir(path: string): void {
			const next = new Set(expandedDirs);
			next.delete(path);
			expandedDirs = next;
		},

		collapseAll(): void {
			expandedDirs = new Set();
		},

		selectFile(path: string): void {
			selectedPath = path;
		},

		isExpanded(path: string): boolean {
			return expandedDirs.has(path);
		}
	};
}
