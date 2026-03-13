<script lang="ts">
	import { getAgentStore } from '$lib/stores/agents';
	import AgentSelector from './AgentSelector.svelte';
	import AgentSplitView from './AgentSplitView.svelte';
	import ChatInput from '$lib/components/chat/ChatInput.svelte';

	const store = getAgentStore();

	let newAgentHovered = $state(false);
	let stackedHovered = $state(false);
	let splitHovered = $state(false);

	function addAgent() {
		store.addAgent();
	}

	function handleSend(message: string) {
		const targetId = store.focusedAgentId;
		if (!targetId) {
			// Auto-create an agent if none exist
			const id = store.addAgent();
			store.sendMessage(id, message);
		} else {
			store.sendMessage(targetId, message);
		}
	}

	let agentCount = $derived(store.agents.length);
	let isStacked = $derived(store.viewMode === 'stacked');
	let focusedLoading = $derived(
		store.focusedAgentId ? store.isLoading(store.focusedAgentId) : false
	);
</script>

<div style="display: flex; flex-direction: column; height: 100%; background: var(--bg-surface); border-right: 1px solid var(--border-default);">
	<!-- Header -->
	<div style="display: flex; align-items: center; height: 40px; padding: 0 12px; border-bottom: 1px solid var(--border-default); gap: 8px; flex-shrink: 0;">
		<svg width="14" height="14" viewBox="0 0 16 16" fill="var(--banana-yellow)" style="flex-shrink: 0;">
			<path d="M8 0a8 8 0 1 0 0 16A8 8 0 0 0 8 0zm0 14.5a6.5 6.5 0 1 1 0-13 6.5 6.5 0 0 1 0 13zM5.5 5a1 1 0 1 1 2 0 1 1 0 0 1-2 0zm3 0a1 1 0 1 1 2 0 1 1 0 0 1-2 0zm-4 5.5a.75.75 0 0 1 .75-.75h5.5a.75.75 0 0 1 0 1.5h-5.5a.75.75 0 0 1-.75-.75z"/>
		</svg>
		<span style="font-weight: 600; font-size: 13px; color: var(--text-primary);">
			Agents
		</span>
		{#if agentCount > 0}
			<span style="font-size: 10px; color: var(--text-muted); background: rgba(255,255,255,0.06); padding: 1px 6px; border-radius: 8px;">
				{agentCount}
			</span>
		{/if}

		<div style="flex: 1;"></div>

		<!-- View mode toggle -->
		{#if agentCount > 1}
			<div style="display: flex; gap: 2px; background: rgba(255,255,255,0.04); border-radius: 5px; padding: 2px;">
				<button
					style="
						padding: 2px 6px; border: none; border-radius: 3px;
						background: {isStacked ? 'rgba(255, 214, 10, 0.15)' : stackedHovered ? 'rgba(255,255,255,0.06)' : 'transparent'};
						color: {isStacked ? 'var(--banana-yellow)' : 'var(--text-muted)'};
						cursor: pointer; font-size: 10px; font-family: var(--font-ui);
						transition: all 0.1s ease;
					"
					onclick={() => store.viewMode = 'stacked'}
					onmouseenter={() => stackedHovered = true}
					onmouseleave={() => stackedHovered = false}
					title="Stacked view"
				>
					<svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
						<rect x="2" y="1" width="12" height="6" rx="1"/>
						<rect x="2" y="9" width="12" height="6" rx="1"/>
					</svg>
				</button>
				<button
					style="
						padding: 2px 6px; border: none; border-radius: 3px;
						background: {!isStacked ? 'rgba(255, 214, 10, 0.15)' : splitHovered ? 'rgba(255,255,255,0.06)' : 'transparent'};
						color: {!isStacked ? 'var(--banana-yellow)' : 'var(--text-muted)'};
						cursor: pointer; font-size: 10px; font-family: var(--font-ui);
						transition: all 0.1s ease;
					"
					onclick={() => store.viewMode = 'split'}
					onmouseenter={() => splitHovered = true}
					onmouseleave={() => splitHovered = false}
					title="Split view"
				>
					<svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
						<rect x="1" y="2" width="6" height="12" rx="1"/>
						<rect x="9" y="2" width="6" height="12" rx="1"/>
					</svg>
				</button>
			</div>
		{/if}

		<!-- New Agent button -->
		<button
			style="
				display: flex; align-items: center; gap: 4px; padding: 4px 8px;
				background: {newAgentHovered ? 'rgba(255, 214, 10, 0.15)' : 'transparent'};
				border: 1px solid {newAgentHovered ? 'var(--banana-yellow)' : 'var(--border-default)'};
				border-radius: 6px;
				color: {newAgentHovered ? 'var(--banana-yellow)' : 'var(--text-secondary)'};
				cursor: pointer; font-size: 11px; font-family: var(--font-ui);
				transition: all 0.15s ease;
				transform: {newAgentHovered ? 'scale(1.04)' : 'scale(1)'};
			"
			onmouseenter={() => newAgentHovered = true}
			onmouseleave={() => newAgentHovered = false}
			onclick={addAgent}
		>
			<span style="font-size: 14px; line-height: 1;">+</span>
			New Agent
		</button>
	</div>

	<!-- Agent tabs -->
	{#if agentCount > 0}
		<div style="border-bottom: 1px solid var(--border-default); flex-shrink: 0;">
			<AgentSelector />
		</div>
	{/if}

	<!-- Chat area -->
	<AgentSplitView />

	<!-- Input area -->
	<ChatInput
		onSend={handleSend}
		disabled={focusedLoading}
		placeholder={agentCount === 0 ? 'Type to start a new agent...' : 'Ask the agent to build something...'}
	/>
</div>
