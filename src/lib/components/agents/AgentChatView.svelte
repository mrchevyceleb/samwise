<script lang="ts">
	import { getAgentStore, type Agent } from '$lib/stores/agents.svelte';
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
	<!-- Messages -->
	<div
		bind:this={scrollContainer}
		style="flex: 1; overflow-y: auto; padding: 12px 14px; display: flex; flex-direction: column; gap: 6px; min-height: 0;"
	>
		{#if messages.length === 0}
			<div style="
				flex: 1; display: flex; flex-direction: column;
				align-items: center; justify-content: center;
				color: var(--text-muted); gap: 12px;
			">
				<div style="
					width: 48px; height: 48px; display: flex; align-items: center; justify-content: center;
					border-radius: 12px; font-size: 22px; font-weight: 700;
					background: linear-gradient(135deg, rgba(255, 214, 10, 0.15), rgba(255, 214, 10, 0.05));
					border: 1px solid rgba(255, 214, 10, 0.2);
					color: var(--banana-yellow);
					animation: agent-bob 4s ease-in-out infinite;
				">
					A
				</div>
				<span style="font-size: 13px;">Send a message to get started.</span>
				<span style="font-size: 11px; opacity: 0.6;">Your agent can read files, write code, and run tools.</span>
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
			padding: 4px 14px; border-top: 1px solid var(--border-default);
			font-size: 11px; color: var(--banana-yellow-dim);
			font-style: italic; flex-shrink: 0;
			display: flex; align-items: center; gap: 6px;
		">
			<div style="width: 5px; height: 5px; border-radius: 50%; background: var(--banana-yellow); animation: agent-pulse 1.2s ease-in-out infinite;"></div>
			{agent.currentActivity}
		</div>
	{/if}
</div>

<svelte:head>
	<style>
		@keyframes agent-bob {
			0%, 100% { transform: translateY(0); }
			50% { transform: translateY(-6px); }
		}
		@keyframes agent-pulse {
			0%, 100% { opacity: 0.4; }
			50% { opacity: 1; }
		}
	</style>
</svelte:head>
