<script lang="ts">
	import { getFileTreeStore, type FileNode } from '$lib/stores/file-tree';
	import FileTreeNode from './FileTreeNode.svelte';

	interface Props {
		onFileClick: (node: FileNode) => void;
		onFileDoubleClick?: (node: FileNode) => void;
	}

	let { onFileClick, onFileDoubleClick }: Props = $props();

	const store = getFileTreeStore();
</script>

<div style="flex: 1; overflow-y: auto; overflow-x: hidden; padding: 2px 0;" role="tree">
	{#if store.isLoading}
		<div style="display: flex; align-items: center; justify-content: center; height: 80px; color: var(--text-muted); font-size: 12px;">
			Loading files...
		</div>
	{:else if store.tree.length === 0}
		<div style="display: flex; align-items: center; justify-content: center; height: 80px; color: var(--text-muted); font-size: 12px;">
			No files found
		</div>
	{:else}
		{#each store.tree as node (node.path)}
			<FileTreeNode
				{node}
				depth={0}
				{onFileClick}
				{onFileDoubleClick}
			/>
		{/each}
	{/if}
</div>
