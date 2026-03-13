<script lang="ts">
	import { getAgentStore } from '$lib/stores/agents.svelte';
	import AgentChatView from './AgentChatView.svelte';

	const store = getAgentStore();

	let visible = $derived(store.visibleAgents);
	let isStacked = $derived(store.viewMode === 'stacked');
</script>

<div style="
	flex: 1; display: flex;
	flex-direction: {isStacked ? 'column' : 'row'};
	overflow: hidden; min-height: 0;
">
	{#if visible.length === 0}
		<div style="
			flex: 1; display: flex; flex-direction: column;
			align-items: center; justify-content: center;
			padding: 24px; color: var(--text-muted);
		">
			<div style="font-size: 40px; animation: bob 4s ease-in-out infinite; margin-bottom: 12px;">
				A
			</div>
			<p style="font-size: 14px; font-weight: 600; color: var(--text-primary); margin-bottom: 4px;">No agents running</p>
			<p style="font-size: 12px; color: var(--text-muted); text-align: center; max-width: 220px; line-height: 1.5;">
				Start a conversation to build, debug, or explore your project.
			</p>
		</div>
	{:else}
		{#each visible as agent, i (agent.id)}
			{#if !isStacked && i > 0}
				<div style="width: 1px; background: var(--border-default); flex-shrink: 0;"></div>
			{/if}
			<div style="
				flex: 1; min-height: {isStacked ? '200px' : '0'};
				min-width: 0; overflow: hidden;
				{isStacked && i > 0 ? 'border-top: 1px solid var(--border-default);' : ''}
			">
				<AgentChatView {agent} />
			</div>
		{/each}
	{/if}
</div>
