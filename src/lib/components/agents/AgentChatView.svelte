<script lang="ts">
	import { getAgentStore, type Agent, type AgentMessage } from '$lib/stores/agents';
	import AgentStatusBadge from './AgentStatusBadge.svelte';
	import ChatMessage from '$lib/components/chat/ChatMessage.svelte';

	interface Props {
		agent: Agent;
	}

	let { agent }: Props = $props();

	const store = getAgentStore();

	let messages = $derived(store.getMessages(agent.id));
	let scrollContainer = $state<HTMLDivElement | null>(null);

	// Auto-scroll to bottom when messages change
	$effect(() => {
		if (messages.length && scrollContainer) {
			requestAnimationFrame(() => {
				if (scrollContainer) {
					scrollContainer.scrollTop = scrollContainer.scrollHeight;
				}
			});
		}
	});
</script>

<div style="display: flex; flex-direction: column; height: 100%; min-height: 0;">
	<!-- Agent header -->
	<div style="
		display: flex; align-items: center; gap: 8px;
		padding: 6px 10px; border-bottom: 1px solid var(--border-default);
		background: rgba(255,255,255,0.02); flex-shrink: 0;
	">
		<span style="font-size: 12px; font-weight: 600; color: var(--text-primary);">
			{agent.name}
		</span>
		<span style="font-size: 10px; color: var(--text-muted); font-family: var(--font-mono);">
			{agent.model}
		</span>
		<div style="margin-left: auto;">
			<AgentStatusBadge status={agent.status} size="md" />
		</div>
	</div>

	<!-- Messages -->
	<div
		bind:this={scrollContainer}
		style="flex: 1; overflow-y: auto; padding: 8px 10px; display: flex; flex-direction: column; gap: 4px; min-height: 0;"
	>
		{#if messages.length === 0}
			<div style="
				flex: 1; display: flex; flex-direction: column;
				align-items: center; justify-content: center;
				color: var(--text-muted); gap: 8px;
			">
				<div style="font-size: 28px; animation: bob 4s ease-in-out infinite;">
					A
				</div>
				<span style="font-size: 12px;">{agent.name} is ready</span>
			</div>
		{:else}
			{#each messages as msg (msg.id)}
				<ChatMessage message={msg} />
			{/each}
		{/if}
	</div>

	<!-- Activity indicator -->
	{#if agent.currentActivity}
		<div style="
			padding: 4px 10px; border-top: 1px solid var(--border-default);
			font-size: 11px; color: var(--banana-yellow-dim);
			font-style: italic; flex-shrink: 0;
		">
			{agent.currentActivity}
		</div>
	{/if}
</div>
