<script lang="ts">
	import { onDestroy } from 'svelte';
	import type { AeTask, Subtask } from '$lib/types';
	import { getTaskStore } from '$lib/stores/tasks.svelte';

	interface Props {
		task: AeTask;
	}

	let { task }: Props = $props();
	const taskStore = getTaskStore();

	let collapsed = $state(false);
	let newSubtaskTitle = $state('');
	let editingId = $state<string | null>(null);
	let editText = $state('');
	let editInputEl = $state<HTMLInputElement | null>(null);
	let dragId = $state<string | null>(null);
	let dropTargetId = $state<string | null>(null);
	let dropPosition = $state<'above' | 'below'>('below');

	// Track active drag listeners for cleanup
	let activeDragCleanup: (() => void) | null = null;

	// Auto-focus action for edit inputs
	function autoFocus(node: HTMLInputElement) {
		node.focus();
		node.select();
	}

	let subtasks = $derived<Subtask[]>(
		(task.subtasks || []).slice().sort((a, b) => a.order - b.order)
	);
	let doneCount = $derived(subtasks.filter(s => s.done).length);
	let totalCount = $derived(subtasks.length);
	let progressPct = $derived(totalCount === 0 ? 0 : Math.round((doneCount / totalCount) * 100));
	let allDone = $derived(totalCount > 0 && doneCount === totalCount);

	// Track recently toggled IDs for the pop animation
	let recentlyToggled = $state<Set<string>>(new Set());
	// Track recently added IDs for the slide-in animation
	let recentlyAdded = $state<Set<string>>(new Set());

	let persistError = $state(false);

	async function persist(updated: Subtask[]) {
		try {
			persistError = false;
			await taskStore.updateTask(task.id, { subtasks: updated });
		} catch {
			persistError = true;
			setTimeout(() => { persistError = false; }, 3000);
		}
	}

	onDestroy(() => {
		if (activeDragCleanup) activeDragCleanup();
	});

	async function toggleSubtask(id: string) {
		const updated = subtasks.map(s =>
			s.id === id ? { ...s, done: !s.done } : s
		);
		recentlyToggled = new Set([...recentlyToggled, id]);
		setTimeout(() => {
			recentlyToggled = new Set([...recentlyToggled].filter(x => x !== id));
		}, 300);
		await persist(updated);
	}

	async function deleteSubtask(id: string) {
		const updated = subtasks.filter(s => s.id !== id);
		await persist(updated);
	}

	async function addSubtask() {
		const title = newSubtaskTitle.trim();
		if (!title) return;
		const newId = crypto.randomUUID();
		const newItem: Subtask = {
			id: newId,
			title,
			done: false,
			order: subtasks.length,
		};
		recentlyAdded = new Set([...recentlyAdded, newId]);
		setTimeout(() => {
			recentlyAdded = new Set([...recentlyAdded].filter(x => x !== newId));
		}, 400);
		newSubtaskTitle = '';
		await persist([...subtasks, newItem]);
	}

	function startEdit(s: Subtask) {
		editingId = s.id;
		editText = s.title;
	}

	async function saveEdit() {
		if (editingId && editText.trim()) {
			const updated = subtasks.map(s =>
				s.id === editingId ? { ...s, title: editText.trim() } : s
			);
			await persist(updated);
		}
		editingId = null;
		editText = '';
	}

	function handleAddKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter') {
			e.preventDefault();
			addSubtask();
		}
	}

	function handleEditKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter') { e.preventDefault(); saveEdit(); }
		if (e.key === 'Escape') { editingId = null; }
	}

	// Drag-and-drop
	let dragStartY = $state(0);

	function onDragStart(e: MouseEvent, id: string) {
		e.preventDefault();
		dragId = id;
		dragStartY = e.clientY;

		function onMove(ev: MouseEvent) {
			// Find which subtask row we're over
			const els = document.querySelectorAll('[data-subtask-id]');
			let closestId: string | null = null;
			let closestPos: 'above' | 'below' = 'below';
			let minDist = Infinity;
			for (const el of els) {
				const rect = el.getBoundingClientRect();
				const mid = rect.top + rect.height / 2;
				const dist = Math.abs(ev.clientY - mid);
				if (dist < minDist) {
					minDist = dist;
					closestId = (el as HTMLElement).dataset.subtaskId!;
					closestPos = ev.clientY < mid ? 'above' : 'below';
				}
			}
			if (closestId && closestId !== dragId) {
				dropTargetId = closestId;
				dropPosition = closestPos;
			} else {
				dropTargetId = null;
			}
		}

		function onUp() {
			if (dragId && dropTargetId) {
				reorderSubtasks(dragId, dropTargetId, dropPosition);
			}
			dragId = null;
			dropTargetId = null;
			cleanup();
		}

		function cleanup() {
			window.removeEventListener('mousemove', onMove);
			window.removeEventListener('mouseup', onUp);
			activeDragCleanup = null;
		}

		activeDragCleanup = cleanup;
		window.addEventListener('mousemove', onMove);
		window.addEventListener('mouseup', onUp);
	}

	async function reorderSubtasks(fromId: string, toId: string, pos: 'above' | 'below') {
		const arr = [...subtasks];
		const fromIdx = arr.findIndex(s => s.id === fromId);
		const item = arr.splice(fromIdx, 1)[0];
		let toIdx = arr.findIndex(s => s.id === toId);
		if (pos === 'below') toIdx += 1;
		arr.splice(toIdx, 0, item);
		const reordered = arr.map((s, i) => ({ ...s, order: i }));
		await persist(reordered);
	}
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<div style="display: flex; flex-direction: column; gap: 8px;">
	<!-- Header row -->
	<div
		style="
			display: flex; align-items: center; gap: 8px;
			cursor: pointer; user-select: none;
		"
		onclick={() => collapsed = !collapsed}
	>
		<!-- Checklist icon -->
		<svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="var(--text-muted)" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
			<rect x="1" y="1" width="14" height="14" rx="2" />
			<path d="M4.5 8.5l2 2 5-5" />
		</svg>
		<span style="
			font-size: 11px; font-weight: 600; color: var(--text-muted);
			text-transform: uppercase; letter-spacing: 0.5px;
		">Checklist</span>

		<!-- Progress count -->
		{#if totalCount > 0}
			<span style="
				font-size: 12px; font-weight: 700; font-family: var(--font-mono);
				color: {allDone ? 'var(--accent-green)' : 'var(--accent-indigo)'};
			">{doneCount}/{totalCount}</span>
		{/if}

		<div style="flex: 1;"></div>

		<!-- Collapse chevron -->
		<svg
			width="14" height="14" viewBox="0 0 16 16" fill="none"
			stroke="var(--text-muted)" stroke-width="1.5" stroke-linecap="round"
			style="
				transition: transform 0.25s cubic-bezier(0.4, 0, 0.2, 1);
				transform: rotate({collapsed ? '-90deg' : '0deg'});
			"
		>
			<path d="M4 6l4 4 4-4" />
		</svg>
	</div>

	<!-- Progress bar -->
	{#if totalCount > 0}
		<div style="
			width: 100%; height: 6px; border-radius: 3px;
			background: var(--bg-primary); overflow: hidden;
		">
			<div style="
				height: 100%; border-radius: 3px;
				width: {progressPct}%;
				background: {allDone ? 'var(--accent-green)' : 'var(--accent-indigo)'};
				transition: width 0.4s cubic-bezier(0.4, 0, 0.2, 1), background 0.3s ease;
			"></div>
		</div>
	{/if}

	{#if persistError}
		<div style="
			font-size: 11px; color: var(--accent-red); padding: 4px 8px;
			background: rgba(248, 81, 73, 0.06); border-radius: 6px;
		">Save failed. Changes may not have persisted.</div>
	{/if}

	<!-- Subtask list + add input (collapsible) -->
	{#if !collapsed}
		<div style="display: flex; flex-direction: column; gap: 2px;">
			{#each subtasks as st (st.id)}
				{@const isDragging = dragId === st.id}
				{@const isDropTarget = dropTargetId === st.id}
				{@const isEditing = editingId === st.id}
				{@const wasToggled = recentlyToggled.has(st.id)}
				{@const wasAdded = recentlyAdded.has(st.id)}
				<div
					data-subtask-id={st.id}
					style="
						display: flex; align-items: center; gap: 8px;
						padding: 6px 4px; border-radius: 6px;
						transition: background 0.1s ease, opacity 0.15s ease, transform 0.15s ease;
						opacity: {isDragging ? '0.5' : '1'};
						transform: {isDragging ? 'scale(0.98)' : 'scale(1)'};
						border-top: {isDropTarget && dropPosition === 'above' ? '2px solid var(--accent-indigo)' : '2px solid transparent'};
						border-bottom: {isDropTarget && dropPosition === 'below' ? '2px solid var(--accent-indigo)' : '2px solid transparent'};
						animation: {wasAdded ? 'subtask-slide-in 0.3s ease' : 'none'};
					"
					onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = 'var(--bg-column-header-hover)'; }}
					onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background = 'transparent'; }}
				>
					<!-- Drag handle -->
					<div
						style="
							display: flex; flex-direction: column; gap: 1.5px;
							cursor: grab; opacity: 0; transition: opacity 0.15s ease;
							padding: 2px;
						"
						class="drag-handle"
						onmousedown={(e) => onDragStart(e, st.id)}
					>
						<svg width="8" height="12" viewBox="0 0 8 12" fill="var(--text-muted)">
							<circle cx="2" cy="1.5" r="1" />
							<circle cx="6" cy="1.5" r="1" />
							<circle cx="2" cy="5.5" r="1" />
							<circle cx="6" cy="5.5" r="1" />
							<circle cx="2" cy="9.5" r="1" />
							<circle cx="6" cy="9.5" r="1" />
						</svg>
					</div>

					<!-- Checkbox -->
					<div
						role="checkbox"
						aria-checked={st.done}
						tabindex="0"
						style="
							width: 18px; height: 18px; border-radius: 4px; flex-shrink: 0;
							border: 1.5px solid {st.done ? 'var(--accent-indigo)' : 'var(--border-bright)'};
							background: {st.done ? 'var(--accent-indigo)' : 'transparent'};
							display: flex; align-items: center; justify-content: center;
							cursor: pointer; transition: all 0.15s ease;
							animation: {wasToggled ? 'check-pop 0.25s ease' : 'none'};
						"
						onclick={() => toggleSubtask(st.id)}
						onkeydown={(e) => { if (e.key === ' ' || e.key === 'Enter') { e.preventDefault(); toggleSubtask(st.id); } }}
					>
						{#if st.done}
							<svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="white" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
								<path d="M2 5.5l2 2 4-4" style="stroke-dasharray: 20; stroke-dashoffset: 0; animation: check-draw 0.3s ease;" />
							</svg>
						{/if}
					</div>

					<!-- Title -->
					{#if isEditing}
						<input
							type="text"
							bind:value={editText}
							bind:this={editInputEl}
							style="
								flex: 1; padding: 2px 6px;
								background: var(--bg-primary); border: 1px solid rgba(99, 102, 241, 0.3);
								border-radius: 6px; color: var(--text-primary);
								font-size: 13px; font-family: var(--font-ui); outline: none;
							"
							onblur={saveEdit}
							onkeydown={handleEditKeydown}
							use:autoFocus
						/>
					{:else}
						<span
							style="
								flex: 1; min-width: 0; font-size: 13px; cursor: pointer;
								color: {st.done ? 'var(--text-muted)' : 'var(--text-secondary)'};
								text-decoration: {st.done ? 'line-through' : 'none'};
								opacity: {st.done ? '0.6' : '1'};
								word-break: break-word; overflow-wrap: anywhere;
								transition: color 0.15s ease, opacity 0.15s ease;
							"
							onclick={() => startEdit(st)}
						>{st.title}</span>
					{/if}

					<!-- Delete button -->
					<button
						style="
							width: 20px; height: 20px; border: none; background: none;
							display: flex; align-items: center; justify-content: center;
							cursor: pointer; border-radius: 4px;
							opacity: 0; transition: opacity 0.15s ease, color 0.1s ease, background 0.1s ease;
							color: var(--text-muted); flex-shrink: 0;
						"
						class="delete-btn"
						onmouseenter={(e) => { const el = e.currentTarget as HTMLElement; el.style.color = 'var(--accent-red)'; el.style.background = 'rgba(248, 81, 73, 0.08)'; }}
						onmouseleave={(e) => { const el = e.currentTarget as HTMLElement; el.style.color = 'var(--text-muted)'; el.style.background = 'none'; }}
						onclick={() => deleteSubtask(st.id)}
					>
						<svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.5">
							<line x1="2" y1="2" x2="8" y2="8" />
							<line x1="8" y1="2" x2="2" y2="8" />
						</svg>
					</button>
				</div>
			{/each}
		</div>

		<!-- Add input -->
		<div style="display: flex; align-items: center; gap: 8px; margin-top: 4px;">
			<input
				type="text"
				bind:value={newSubtaskTitle}
				placeholder="Add a subtask..."
				style="
					flex: 1; padding: 7px 12px;
					background: var(--bg-primary); border: 1px solid rgba(99, 102, 241, 0.3);
					border-radius: 8px; color: var(--text-primary);
					font-size: 13px; font-family: var(--font-ui); outline: none;
					transition: border-color 0.15s ease;
				"
				onfocus={(e) => { (e.currentTarget as HTMLElement).style.borderColor = 'rgba(99, 102, 241, 0.5)'; }}
				onblur={(e) => { (e.currentTarget as HTMLElement).style.borderColor = 'rgba(99, 102, 241, 0.3)'; }}
				onkeydown={handleAddKeydown}
			/>
		</div>
	{/if}
</div>

<style>
	/* Show drag handle and delete button on row hover */
	div[data-subtask-id]:hover :global(.drag-handle) {
		opacity: 1 !important;
	}
	div[data-subtask-id]:hover :global(.delete-btn) {
		opacity: 1 !important;
	}
</style>
