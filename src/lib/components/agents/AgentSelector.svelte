<script lang="ts">
	import { getAgentStore, type Agent } from '$lib/stores/agents.svelte';
	import AgentStatusBadge from './AgentStatusBadge.svelte';

	const store = getAgentStore();

	let hoveredId = $state<string | null>(null);
	let closeHoveredId = $state<string | null>(null);

	function focusAgent(id: string) {
		store.focusedAgentId = id;
	}

	function closeAgent(e: MouseEvent, id: string) {
		e.stopPropagation();
		store.removeAgent(id);
	}

	const statusColors: Record<Agent['status'], string> = {
		idle: '#3FB950',
		thinking: '#FFD60A',
		writing: '#58A6FF',
		running_tool: '#D29922',
		done: '#3FB950',
		error: '#F85149'
	};
</script>

<div style="display: flex; align-items: center; gap: 2px; overflow-x: auto; padding: 0 4px; min-height: 30px;">
	{#each store.agents as agent (agent.id)}
		{@const isFocused = store.focusedAgentId === agent.id}
		{@const isHovered = hoveredId === agent.id}
		<div
			style="
				display: flex; align-items: center; gap: 6px;
				padding: 3px 8px; border-radius: 6px;
				background: {isFocused ? 'rgba(255, 214, 10, 0.12)' : isHovered ? 'rgba(255,255,255,0.04)' : 'transparent'};
				border: 1px solid {isFocused ? 'rgba(255, 214, 10, 0.3)' : 'transparent'};
				color: {isFocused ? 'var(--text-primary)' : 'var(--text-secondary)'};
				cursor: pointer; font-family: var(--font-ui); font-size: 11px;
				font-weight: {isFocused ? '600' : '400'};
				transition: all 0.12s ease; white-space: nowrap; flex-shrink: 0;
			"
			role="tab"
			tabindex="0"
			aria-selected={isFocused}
			onclick={() => focusAgent(agent.id)}
			onkeydown={(e: KeyboardEvent) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); focusAgent(agent.id); }}}
			onmouseenter={() => hoveredId = agent.id}
			onmouseleave={() => hoveredId = null}
		>
			<AgentStatusBadge status={agent.status} size="sm" />
			<span>{agent.name}</span>
			<button
				style="
					display: flex; align-items: center; justify-content: center;
					width: 14px; height: 14px; border-radius: 3px; border: none; padding: 0;
					font-size: 10px; line-height: 1; color: var(--text-muted);
					background: {closeHoveredId === agent.id ? 'rgba(248, 81, 73, 0.2)' : 'transparent'};
					transition: all 0.1s ease; cursor: pointer; font-family: var(--font-ui);
				"
				aria-label="Close agent"
				onclick={(e: MouseEvent) => closeAgent(e, agent.id)}
				onmouseenter={() => closeHoveredId = agent.id}
				onmouseleave={() => closeHoveredId = null}
			>
				x
			</button>
		</div>
	{/each}
</div>
