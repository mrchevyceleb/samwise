<script lang="ts">
	import { getAgentStore } from '$lib/stores/agents.svelte';
	import { getLayout } from '$lib/stores/layout.svelte';
	import AgentSelector from './AgentSelector.svelte';
	import AgentSplitView from './AgentSplitView.svelte';
	import ChatInput from '$lib/components/chat/ChatInput.svelte';
	import ClaudeCodePanel from '$lib/components/claude-code/ClaudeCodePanel.svelte';

	const store = getAgentStore();
	const layout = getLayout();

	let newAgentHovered = $state(false);
	let stackedHovered = $state(false);
	let splitHovered = $state(false);

	let isClaudeCode = $derived(layout.chatMode === 'claude-code');

	function addAgent() {
		store.addAgent();
	}

	function handleSend(message: string) {
		const targetId = store.focusedAgentId;
		if (!targetId) {
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
	<!-- Mode toggle header -->
	<div style="display: flex; align-items: center; height: 40px; padding: 0 12px; border-bottom: 1px solid var(--border-default); gap: 8px; flex-shrink: 0;">
		<!-- Mode toggle: Agent / Claude Code -->
		<div style="display: flex; gap: 0; background: rgba(255,255,255,0.04); border-radius: 6px; padding: 2px; border: 1px solid var(--border-default);">
			<button
				style="
					padding: 3px 10px; border: none; border-radius: 4px;
					background: {!isClaudeCode ? 'rgba(255, 214, 10, 0.15)' : 'transparent'};
					color: {!isClaudeCode ? 'var(--banana-yellow)' : 'var(--text-muted)'};
					cursor: pointer; font-size: 11px; font-family: var(--font-ui); font-weight: 500;
					transition: all 0.12s ease;
				"
				onclick={() => layout.chatMode = 'agent'}
			>
				Agent
			</button>
			<button
				style="
					padding: 3px 10px; border: none; border-radius: 4px;
					background: {isClaudeCode ? 'rgba(255, 214, 10, 0.15)' : 'transparent'};
					color: {isClaudeCode ? 'var(--banana-yellow)' : 'var(--text-muted)'};
					cursor: pointer; font-size: 11px; font-family: var(--font-ui); font-weight: 500;
					transition: all 0.12s ease;
				"
				onclick={() => layout.chatMode = 'claude-code'}
			>
				Claude Code
			</button>
		</div>

		<div style="flex: 1;"></div>

		{#if !isClaudeCode}
			<!-- View mode toggle (agent mode only) -->
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
		{/if}
	</div>

	{#if isClaudeCode}
		<!-- Claude Code mode -->
		<ClaudeCodePanel />
	{:else}
		<!-- Agent mode -->
		{#if agentCount > 0}
			<div style="border-bottom: 1px solid var(--border-default); flex-shrink: 0;">
				<AgentSelector />
			</div>
		{/if}

		<AgentSplitView />

		<ChatInput
			onSend={handleSend}
			disabled={focusedLoading}
			placeholder={agentCount === 0 ? 'Type to start a new agent...' : 'Ask the agent to build something...'}
		/>
	{/if}
</div>
