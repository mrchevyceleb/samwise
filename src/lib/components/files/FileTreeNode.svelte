<script lang="ts">
	import { getFileTreeStore, type FileNode } from '$lib/stores/file-tree.svelte';
	import FileTreeNode from './FileTreeNode.svelte';

	interface Props {
		node: FileNode;
		depth: number;
		onFileClick: (node: FileNode) => void;
		onFileDoubleClick?: (node: FileNode) => void;
	}

	let { node, depth, onFileClick, onFileDoubleClick }: Props = $props();

	const store = getFileTreeStore();

	let hovered = $state(false);
	let expanded = $derived(store.isExpanded(node.path));
	let isSelected = $derived(store.selectedPath === node.path);

	const extColors: Record<string, string> = {
		ts: '#3178C6',
		tsx: '#3178C6',
		js: '#F7DF1E',
		jsx: '#F7DF1E',
		svelte: '#FF3E00',
		css: '#264DE4',
		html: '#E34C26',
		json: '#A8B9CC',
		md: '#083FA1',
		rs: '#DEA584',
		py: '#3572A5',
		go: '#00ADD8',
		toml: '#9C4121',
		yaml: '#CB171E',
		yml: '#CB171E',
		lock: '#6E7681',
		png: '#A855F7',
		jpg: '#A855F7',
		svg: '#FFB13B',
		gif: '#A855F7',
		woff2: '#A855F7',
		sh: '#4EAA25',
		bat: '#C1F12E'
	};

	function getColor(): string {
		if (node.is_dir) return '#FFD60A';
		return extColors[node.ext || ''] || '#8B949E';
	}

	function handleClick() {
		if (node.is_dir) {
			store.toggleDir(node.path);
		} else {
			store.selectFile(node.path);
			onFileClick(node);
		}
	}

	function handleDoubleClick() {
		if (!node.is_dir) {
			onFileDoubleClick?.(node);
		}
	}

	function handleContextMenu(e: MouseEvent) {
		e.preventDefault();
		// Placeholder for context menu
	}

	let paddingLeft = $derived(`${depth * 16 + 8}px`);
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<div
	style="
		display: flex; align-items: center; gap: 4px;
		padding: 3px 8px 3px {paddingLeft};
		cursor: pointer; font-size: 12px; white-space: nowrap;
		background: {isSelected ? 'rgba(255, 214, 10, 0.1)' : hovered ? 'rgba(255,255,255,0.04)' : 'transparent'};
		color: {isSelected ? 'var(--text-primary)' : 'var(--text-secondary)'};
		border-left: {isSelected ? '2px solid var(--banana-yellow)' : '2px solid transparent'};
		transition: background 0.1s ease, color 0.1s ease;
		user-select: none;
	"
	role="treeitem"
	tabindex="-1"
	aria-selected={isSelected}
	aria-expanded={node.is_dir ? expanded : undefined}
	onclick={handleClick}
	ondblclick={handleDoubleClick}
	oncontextmenu={handleContextMenu}
	onmouseenter={() => hovered = true}
	onmouseleave={() => hovered = false}
	draggable="true"
>
	<!-- Expand chevron for directories -->
	{#if node.is_dir}
		<span style="
			font-size: 8px; width: 12px; display: inline-flex;
			align-items: center; justify-content: center;
			color: var(--text-muted); transition: transform 0.15s ease;
			transform: rotate({expanded ? '90deg' : '0deg'});
		">
			&#9654;
		</span>
	{:else}
		<span style="width: 12px; display: inline-block;"></span>
	{/if}

	<!-- Icon -->
	{#if node.is_dir}
		<span style="font-size: 13px; color: {getColor()}; flex-shrink: 0;">
			{expanded ? '&#128194;' : '&#128193;'}
		</span>
	{:else}
		<svg width="13" height="13" viewBox="0 0 16 16" fill={getColor()} style="flex-shrink: 0; opacity: 0.8;">
			<path d="M3.75 1.5a.25.25 0 0 0-.25.25v12.5c0 .138.112.25.25.25h8.5a.25.25 0 0 0 .25-.25V5.5l-4-4h-5z"/>
		</svg>
	{/if}

	<!-- Name -->
	<span style="
		overflow: hidden; text-overflow: ellipsis;
		{node.is_dir ? 'font-weight: 500;' : ''}
	">
		{node.name}
	</span>

	<!-- Extension badge for files -->
	{#if !node.is_dir && node.ext && hovered}
		<span style="
			margin-left: auto; font-size: 9px; padding: 0 4px;
			border-radius: 3px; background: rgba(255,255,255,0.06);
			color: var(--text-muted); text-transform: uppercase;
			letter-spacing: 0.3px;
		">
			{node.ext}
		</span>
	{/if}
</div>

<!-- Children (recursive) -->
{#if node.is_dir && expanded && node.children}
	{#each node.children as child (child.path)}
		<FileTreeNode
			node={child}
			depth={depth + 1}
			{onFileClick}
			{onFileDoubleClick}
		/>
	{/each}
{/if}
