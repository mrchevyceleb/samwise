<script lang="ts">
	import type { AeTask, TaskStatus, TaskPriority } from '$lib/types';
	import { KANBAN_COLUMNS } from '$lib/types';
	import { getTaskStore } from '$lib/stores/tasks.svelte';
	import { getTheme } from '$lib/stores/theme.svelte';

	interface Props {
		task: AeTask;
		x: number;
		y: number;
		onClose: () => void;
		onOpenDetail: (task: AeTask) => void;
	}

	let { task, x, y, onClose, onOpenDetail }: Props = $props();
	const taskStore = getTaskStore();
	const theme = getTheme();

	let submenu = $state<'status' | 'priority' | 'assignee' | null>(null);
	let confirmDelete = $state(false);
	let menuEl = $state<HTMLDivElement | null>(null);

	// Clamp position so menu stays on-screen
	let posX = $derived(Math.min(x, window.innerWidth - 220));
	let posY = $derived(Math.min(y, window.innerHeight - 360));

	const priorities: { value: TaskPriority; label: string; color: string }[] = [
		{ value: 'critical', label: 'Critical', color: '#f85149' },
		{ value: 'high', label: 'High', color: '#d29922' },
		{ value: 'medium', label: 'Medium', color: '#6366f1' },
		{ value: 'low', label: 'Low', color: '#6e7681' },
	];

	async function moveToStatus(status: TaskStatus) {
		await taskStore.moveTask(task.id, status);
		onClose();
	}

	async function changePriority(p: TaskPriority) {
		await taskStore.updateTask(task.id, { priority: p });
		onClose();
	}

	async function changeAssignee(a: string) {
		await taskStore.updateTask(task.id, { assignee: a });
		onClose();
	}

	async function handleDelete() {
		if (!confirmDelete) {
			confirmDelete = true;
			setTimeout(() => confirmDelete = false, 2000);
			return;
		}
		await taskStore.deleteTask(task.id);
		onClose();
	}

	async function toggleHold() {
		await taskStore.updateTask(task.id, { on_hold: !task.on_hold });
		onClose();
	}

	function openDetails() {
		onOpenDetail(task);
		onClose();
	}

	function handleBackdropClick() {
		onClose();
	}
</script>

<svelte:window onkeydown={(e) => { if (e.key === 'Escape') onClose(); }} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<div
	style="position: fixed; inset: 0; z-index: 300;"
	onclick={handleBackdropClick}
	oncontextmenu={(e) => { e.preventDefault(); handleBackdropClick(); }}
>
	<div
		bind:this={menuEl}
		style="
			position: fixed; left: {posX}px; top: {posY}px;
			min-width: 200px; z-index: 301;
			background: {theme.c.gradientModal};
			border: 1px solid {theme.c.borderGlow};
			border-radius: 10px;
			box-shadow: 0 12px 40px rgba(0,0,0,0.4), 0 0 20px rgba(99,102,241,0.06);
			padding: 6px;
			animation: spring-in 0.12s ease;
		"
		onclick={(e) => e.stopPropagation()}
		oncontextmenu={(e) => { e.preventDefault(); e.stopPropagation(); }}
	>
		<!-- Open Details -->
		<button
			class="ctx-item"
			style="
				display: flex; align-items: center; gap: 8px; width: 100%;
				padding: 7px 10px; border: none; background: none; border-radius: 6px;
				color: var(--text-primary); font-size: 12px; font-weight: 500;
				font-family: var(--font-ui); cursor: pointer; text-align: left;
				transition: background 0.1s;
			"
			onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = 'var(--bg-column-header-hover)'; submenu = null; }}
			onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background = 'none'; }}
			onclick={openDetails}
		>
			<svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor" style="opacity: 0.6;">
				<path d="M0 1.75A.75.75 0 01.75 1h4.253c.456 0 .89.181 1.212.503l6.53 6.53a.75.75 0 010 1.06l-4.253 4.254a.75.75 0 01-1.06 0l-6.53-6.53A1.714 1.714 0 01.75 5.63V1.75z"/>
			</svg>
			Open Details
		</button>

		<div style="height: 1px; background: var(--border-subtle); margin: 4px 6px;"></div>

		<!-- Move to Status (submenu) -->
		<div style="position: relative;">
			<button
				class="ctx-item"
				style="
					display: flex; align-items: center; gap: 8px; width: 100%;
					padding: 7px 10px; border: none; border-radius: 6px;
					background: {submenu === 'status' ? 'var(--bg-column-header-hover)' : 'none'};
					color: var(--text-primary); font-size: 12px; font-weight: 500;
					font-family: var(--font-ui); cursor: pointer; text-align: left;
					transition: background 0.1s;
				"
				onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = 'var(--bg-column-header-hover)'; submenu = 'status'; }}
				onmouseleave={(e) => { if (submenu !== 'status') (e.currentTarget as HTMLElement).style.background = 'none'; }}
			>
				<svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor" style="opacity: 0.6;">
					<path d="M1.5 3.25a2.25 2.25 0 113 2.122v5.256a2.251 2.251 0 11-1.5 0V5.372A2.25 2.25 0 011.5 3.25zm5.677-.177L9.573.677A.25.25 0 0110 .854v4.792a.25.25 0 01-.427.177L7.177 3.427a.25.25 0 010-.354z"/>
				</svg>
				<span style="flex: 1;">Move to...</span>
				<svg width="8" height="8" viewBox="0 0 8 8" fill="currentColor" style="opacity: 0.4;"><path d="M2 1l4 3-4 3z"/></svg>
			</button>

			{#if submenu === 'status'}
				<div
					style="
						position: absolute; left: 100%; top: -6px;
						min-width: 160px; padding: 6px;
						background: {theme.c.gradientModal};
						border: 1px solid {theme.c.borderGlow};
						border-radius: 10px;
						box-shadow: 0 8px 30px rgba(0,0,0,0.3);
						margin-left: 4px;
					"
					onmouseenter={() => submenu = 'status'}
				>
					{#each KANBAN_COLUMNS as col}
						<button
							style="
								display: flex; align-items: center; gap: 8px; width: 100%;
								padding: 6px 10px; border: none; border-radius: 6px;
								background: {task.status === col.status ? col.color + '12' : 'none'};
								color: {task.status === col.status ? col.color : 'var(--text-primary)'};
								font-size: 12px; font-weight: {task.status === col.status ? '700' : '500'};
								font-family: var(--font-ui); cursor: pointer; text-align: left;
								transition: background 0.1s;
							"
							onmouseenter={(e) => { if (task.status !== col.status) (e.currentTarget as HTMLElement).style.background = 'var(--bg-column-header-hover)'; }}
							onmouseleave={(e) => { if (task.status !== col.status) (e.currentTarget as HTMLElement).style.background = 'none'; }}
							onclick={() => moveToStatus(col.status)}
						>
							<span style="width: 7px; height: 7px; border-radius: 50%; background: {col.color}; flex-shrink: 0;"></span>
							{col.label}
							{#if task.status === col.status}
								<svg width="10" height="10" viewBox="0 0 16 16" fill="currentColor" style="margin-left: auto; opacity: 0.7;"><path d="M13.78 4.22a.75.75 0 010 1.06l-7.25 7.25a.75.75 0 01-1.06 0L2.22 9.28a.75.75 0 011.06-1.06L6 10.94l6.72-6.72a.75.75 0 011.06 0z"/></svg>
							{/if}
						</button>
					{/each}
				</div>
			{/if}
		</div>

		<!-- Priority (submenu) -->
		<div style="position: relative;">
			<button
				class="ctx-item"
				style="
					display: flex; align-items: center; gap: 8px; width: 100%;
					padding: 7px 10px; border: none; border-radius: 6px;
					background: {submenu === 'priority' ? 'var(--bg-column-header-hover)' : 'none'};
					color: var(--text-primary); font-size: 12px; font-weight: 500;
					font-family: var(--font-ui); cursor: pointer; text-align: left;
					transition: background 0.1s;
				"
				onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = 'var(--bg-column-header-hover)'; submenu = 'priority'; }}
				onmouseleave={(e) => { if (submenu !== 'priority') (e.currentTarget as HTMLElement).style.background = 'none'; }}
			>
				<svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor" style="opacity: 0.6;">
					<path d="M8 1.5a.5.5 0 01.5.5v5h5a.5.5 0 010 1h-5v5a.5.5 0 01-1 0V8h-5a.5.5 0 010-1h5V2a.5.5 0 01.5-.5z"/>
				</svg>
				<span style="flex: 1;">Priority</span>
				<span style="font-size: 10px; color: var(--text-muted); text-transform: uppercase;">{task.priority}</span>
				<svg width="8" height="8" viewBox="0 0 8 8" fill="currentColor" style="opacity: 0.4;"><path d="M2 1l4 3-4 3z"/></svg>
			</button>

			{#if submenu === 'priority'}
				<div
					style="
						position: absolute; left: 100%; top: -6px;
						min-width: 150px; padding: 6px;
						background: {theme.c.gradientModal};
						border: 1px solid {theme.c.borderGlow};
						border-radius: 10px;
						box-shadow: 0 8px 30px rgba(0,0,0,0.3);
						margin-left: 4px;
					"
					onmouseenter={() => submenu = 'priority'}
				>
					{#each priorities as p}
						<button
							style="
								display: flex; align-items: center; gap: 8px; width: 100%;
								padding: 6px 10px; border: none; border-radius: 6px;
								background: {task.priority === p.value ? p.color + '12' : 'none'};
								color: {task.priority === p.value ? p.color : 'var(--text-primary)'};
								font-size: 12px; font-weight: {task.priority === p.value ? '700' : '500'};
								font-family: var(--font-ui); cursor: pointer; text-align: left;
								transition: background 0.1s;
							"
							onmouseenter={(e) => { if (task.priority !== p.value) (e.currentTarget as HTMLElement).style.background = 'var(--bg-column-header-hover)'; }}
							onmouseleave={(e) => { if (task.priority !== p.value) (e.currentTarget as HTMLElement).style.background = 'none'; }}
							onclick={() => changePriority(p.value)}
						>
							<span style="width: 7px; height: 7px; border-radius: 50%; background: {p.color}; flex-shrink: 0;"></span>
							{p.label}
							{#if task.priority === p.value}
								<svg width="10" height="10" viewBox="0 0 16 16" fill="currentColor" style="margin-left: auto; opacity: 0.7;"><path d="M13.78 4.22a.75.75 0 010 1.06l-7.25 7.25a.75.75 0 01-1.06 0L2.22 9.28a.75.75 0 011.06-1.06L6 10.94l6.72-6.72a.75.75 0 011.06 0z"/></svg>
							{/if}
						</button>
					{/each}
				</div>
			{/if}
		</div>

		<!-- Assignee (submenu) -->
		<div style="position: relative;">
			<button
				class="ctx-item"
				style="
					display: flex; align-items: center; gap: 8px; width: 100%;
					padding: 7px 10px; border: none; border-radius: 6px;
					background: {submenu === 'assignee' ? 'var(--bg-column-header-hover)' : 'none'};
					color: var(--text-primary); font-size: 12px; font-weight: 500;
					font-family: var(--font-ui); cursor: pointer; text-align: left;
					transition: background 0.1s;
				"
				onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = 'var(--bg-column-header-hover)'; submenu = 'assignee'; }}
				onmouseleave={(e) => { if (submenu !== 'assignee') (e.currentTarget as HTMLElement).style.background = 'none'; }}
			>
				<svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor" style="opacity: 0.6;">
					<path d="M8 8a3 3 0 100-6 3 3 0 000 6zm-5 6s-1 0-1-1 1-4 6-4 6 3 6 4-1 1-1 1H3z"/>
				</svg>
				<span style="flex: 1;">Assignee</span>
				<span style="font-size: 10px; color: var(--text-muted);">{task.assignee === 'agent' ? 'Agent' : 'Matt'}</span>
				<svg width="8" height="8" viewBox="0 0 8 8" fill="currentColor" style="opacity: 0.4;"><path d="M2 1l4 3-4 3z"/></svg>
			</button>

			{#if submenu === 'assignee'}
				<div
					style="
						position: absolute; left: 100%; top: -6px;
						min-width: 140px; padding: 6px;
						background: {theme.c.gradientModal};
						border: 1px solid {theme.c.borderGlow};
						border-radius: 10px;
						box-shadow: 0 8px 30px rgba(0,0,0,0.3);
						margin-left: 4px;
					"
					onmouseenter={() => submenu = 'assignee'}
				>
					{#each [{ value: 'agent', label: 'Agent', icon: 'robot' }, { value: 'matt', label: 'Matt', icon: 'user' }] as a}
						<button
							style="
								display: flex; align-items: center; gap: 8px; width: 100%;
								padding: 6px 10px; border: none; border-radius: 6px;
								background: {task.assignee === a.value ? 'rgba(99, 102, 241, 0.12)' : 'none'};
								color: {task.assignee === a.value ? 'var(--accent-indigo)' : 'var(--text-primary)'};
								font-size: 12px; font-weight: {task.assignee === a.value ? '700' : '500'};
								font-family: var(--font-ui); cursor: pointer; text-align: left;
								transition: background 0.1s;
							"
							onmouseenter={(e) => { if (task.assignee !== a.value) (e.currentTarget as HTMLElement).style.background = 'var(--bg-column-header-hover)'; }}
							onmouseleave={(e) => { if (task.assignee !== a.value) (e.currentTarget as HTMLElement).style.background = 'none'; }}
							onclick={() => changeAssignee(a.value)}
						>
							{#if a.icon === 'robot'}
								<svg width="11" height="11" viewBox="0 0 16 16" fill="currentColor"><path d="M8 0a1 1 0 011 1v1.07A6.002 6.002 0 0114 8v3a2 2 0 01-2 2H4a2 2 0 01-2-2V8a6.002 6.002 0 015-5.93V1a1 1 0 011-1zM6 9a1 1 0 100 2 1 1 0 000-2zm4 0a1 1 0 100 2 1 1 0 000-2z"/></svg>
							{:else}
								<svg width="11" height="11" viewBox="0 0 16 16" fill="currentColor"><path d="M8 8a3 3 0 100-6 3 3 0 000 6zm-5 6s-1 0-1-1 1-4 6-4 6 3 6 4-1 1-1 1H3z"/></svg>
							{/if}
							{a.label}
							{#if task.assignee === a.value}
								<svg width="10" height="10" viewBox="0 0 16 16" fill="currentColor" style="margin-left: auto; opacity: 0.7;"><path d="M13.78 4.22a.75.75 0 010 1.06l-7.25 7.25a.75.75 0 01-1.06 0L2.22 9.28a.75.75 0 011.06-1.06L6 10.94l6.72-6.72a.75.75 0 011.06 0z"/></svg>
							{/if}
						</button>
					{/each}
				</div>
			{/if}
		</div>

		<div style="height: 1px; background: var(--border-subtle); margin: 4px 6px;"></div>

		<!-- Hold / Release (only meaningful while queued) -->
		{#if task.status === 'queued'}
			<button
				class="ctx-item"
				style="
					display: flex; align-items: center; gap: 8px; width: 100%;
					padding: 7px 10px; border: none; background: none; border-radius: 6px;
					color: {task.on_hold ? 'var(--accent-green)' : 'var(--accent-amber, #d29922)'};
					font-size: 12px; font-weight: 500;
					font-family: var(--font-ui); cursor: pointer; text-align: left;
					transition: background 0.1s;
				"
				onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = task.on_hold ? 'rgba(63, 185, 80, 0.08)' : 'rgba(210, 153, 34, 0.10)'; submenu = null; }}
				onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background = 'none'; }}
				onclick={toggleHold}
			>
				<svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor" style="opacity: 0.75;">
					{#if task.on_hold}
						<path d="M5 2.75C5 1.784 5.784 1 6.75 1h2.5c.966 0 1.75.784 1.75 1.75v10.5A1.75 1.75 0 019.25 15h-2.5A1.75 1.75 0 015 13.25V2.75z"/>
					{:else}
						<path d="M11.5 1.75C11.5 .784 12.284 0 13.25 0a1.75 1.75 0 011.75 1.75v12.5A1.75 1.75 0 0113.25 16a1.75 1.75 0 01-1.75-1.75V1.75zm-7 0C4.5.784 5.284 0 6.25 0A1.75 1.75 0 018 1.75v12.5A1.75 1.75 0 016.25 16 1.75 1.75 0 014.5 14.25V1.75z"/>
					{/if}
				</svg>
				{task.on_hold ? 'Release (Sam can claim)' : 'Hold (Sam will skip)'}
			</button>
			<div style="height: 1px; background: var(--border-subtle); margin: 4px 6px;"></div>
		{/if}

		<!-- Re-queue (only if failed) -->
		{#if task.status === 'failed'}
			<button
				class="ctx-item"
				style="
					display: flex; align-items: center; gap: 8px; width: 100%;
					padding: 7px 10px; border: none; background: none; border-radius: 6px;
					color: var(--accent-indigo); font-size: 12px; font-weight: 500;
					font-family: var(--font-ui); cursor: pointer; text-align: left;
					transition: background 0.1s;
				"
				onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = 'rgba(99, 102, 241, 0.08)'; submenu = null; }}
				onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background = 'none'; }}
				onclick={() => moveToStatus('queued')}
			>
				<svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor" style="opacity: 0.7;">
					<path d="M1.705 8.005a.75.75 0 01.834.656 5.5 5.5 0 009.592 2.97l-1.204-1.204a.25.25 0 01.177-.427h3.646a.25.25 0 01.25.25v3.646a.25.25 0 01-.427.177l-1.38-1.38A7.002 7.002 0 011.05 8.84a.75.75 0 01.656-.834zM8 2.5a5.487 5.487 0 00-4.131 1.869l1.204 1.204A.25.25 0 014.896 6H1.25A.25.25 0 011 5.75V2.104a.25.25 0 01.427-.177l1.38 1.38A7.002 7.002 0 0114.95 7.16a.75.75 0 11-1.49.178A5.5 5.5 0 008 2.5z"/>
				</svg>
				Re-queue
			</button>
		{/if}

		<!-- Copy ID -->
		<button
			class="ctx-item"
			style="
				display: flex; align-items: center; gap: 8px; width: 100%;
				padding: 7px 10px; border: none; background: none; border-radius: 6px;
				color: var(--text-primary); font-size: 12px; font-weight: 500;
				font-family: var(--font-ui); cursor: pointer; text-align: left;
				transition: background 0.1s;
			"
			onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = 'var(--bg-column-header-hover)'; submenu = null; }}
			onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background = 'none'; }}
			onclick={() => { navigator.clipboard.writeText(task.id); onClose(); }}
		>
			<svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor" style="opacity: 0.6;">
				<path d="M0 6.75C0 5.784.784 5 1.75 5h1.5a.75.75 0 010 1.5h-1.5a.25.25 0 00-.25.25v7.5c0 .138.112.25.25.25h7.5a.25.25 0 00.25-.25v-1.5a.75.75 0 011.5 0v1.5A1.75 1.75 0 019.25 16h-7.5A1.75 1.75 0 010 14.25v-7.5z"/>
				<path d="M5 1.75C5 .784 5.784 0 6.75 0h7.5C15.216 0 16 .784 16 1.75v7.5A1.75 1.75 0 0114.25 11h-7.5A1.75 1.75 0 015 9.25v-7.5zm1.75-.25a.25.25 0 00-.25.25v7.5c0 .138.112.25.25.25h7.5a.25.25 0 00.25-.25v-7.5a.25.25 0 00-.25-.25h-7.5z"/>
			</svg>
			Copy Task ID
		</button>

		<div style="height: 1px; background: var(--border-subtle); margin: 4px 6px;"></div>

		<!-- Delete -->
		<button
			class="ctx-item"
			style="
				display: flex; align-items: center; gap: 8px; width: 100%;
				padding: 7px 10px; border: none; background: none; border-radius: 6px;
				color: var(--accent-red); font-size: 12px; font-weight: {confirmDelete ? '700' : '500'};
				font-family: var(--font-ui); cursor: pointer; text-align: left;
				transition: background 0.1s;
			"
			onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = 'rgba(248, 81, 73, 0.08)'; submenu = null; }}
			onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background = 'none'; }}
			onclick={handleDelete}
		>
			<svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor" style="opacity: 0.7;">
				<path d="M6.5 1.75a.25.25 0 01.25-.25h2.5a.25.25 0 01.25.25V3h-3V1.75zm4.5 0V3h2.25a.75.75 0 010 1.5H2.75a.75.75 0 010-1.5H5V1.75C5 .784 5.784 0 6.75 0h2.5C10.216 0 11 .784 11 1.75zM4.496 6.675a.75.75 0 10-1.492.15l.66 6.6A1.75 1.75 0 005.405 15h5.19a1.75 1.75 0 001.741-1.575l.66-6.6a.75.75 0 00-1.492-.15l-.66 6.6a.25.25 0 01-.249.225h-5.19a.25.25 0 01-.249-.225l-.66-6.6z"/>
			</svg>
			{confirmDelete ? 'Click again to confirm' : 'Delete Task'}
		</button>
	</div>
</div>
