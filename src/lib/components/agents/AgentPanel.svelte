<script lang="ts">
	import { getAgentStore } from '$lib/stores/agents.svelte';
	import { getClaudeCodeStore } from '$lib/stores/claude-code.svelte';
	import { getLayout } from '$lib/stores/layout.svelte';
	import { getWorkspace } from '$lib/stores/workspace.svelte';
	import ConversationSidebar from './ConversationSidebar.svelte';
	import ChatHeader from './ChatHeader.svelte';
	import AgentChatView from './AgentChatView.svelte';
	import ChatInput from '$lib/components/chat/ChatInput.svelte';
	import ClaudeCodeChatView from '$lib/components/claude-code/ClaudeCodeChatView.svelte';
	import ClaudeCodeInput from '$lib/components/claude-code/ClaudeCodeInput.svelte';

	const agents = getAgentStore();
	const cc = getClaudeCodeStore();
	const layout = getLayout();
	const workspace = getWorkspace();

	let focused = $derived(layout.focusedConversation);
	let focusedAgent = $derived(focused?.type === 'agent' ? agents.getAgent(focused.id) : undefined);
	let ccSession = $derived(focused?.type === 'claude-code' ? cc.getSession(focused.id) : undefined);
	let agentLoading = $derived(
		focusedAgent ? agents.isLoading(focusedAgent.id) : false
	);
	let ccRunning = $derived(ccSession?.status === 'running');

	let agentModelName = $derived((() => {
		if (!focusedAgent) return '';
		return focusedAgent.model.split('/').pop() || focusedAgent.model;
	})());

	// Agent handlers
	function handleAgentSend(message: string) {
		if (!focused || focused.type !== 'agent') {
			// Create new agent
			const id = agents.addAgent();
			layout.focusedConversation = { id, type: 'agent' };
			agents.sendMessage(id, message);
			return;
		}
		agents.sendMessage(focused.id, message);
	}

	// Claude Code handlers
	function handleCCSend(message: string) {
		if (!ccSession) return;
		cc.sendMessage(ccSession.id, message);
	}

	function handleCCStop() {
		if (ccSession) cc.stopSession(ccSession.id);
	}

	function handleCCSteer(steer: string) {
		if (!ccSession) return;
		const sessionId = ccSession.id;
		cc.stopSession(sessionId).then(() => {
			setTimeout(() => {
				cc.sendMessage(sessionId, steer);
			}, 200);
		});
	}

</script>

<div style="display: flex; height: 100%; background: var(--bg-surface);">
	<ConversationSidebar />

	<div style="flex: 1; display: flex; flex-direction: column; min-width: 0;">
		<ChatHeader />

		{#if focused?.type === 'claude-code' && ccSession}
			<!-- Claude Code chat view -->
			<div style="flex: 1; min-height: 0;">
				<ClaudeCodeChatView sessionId={ccSession.id} />
			</div>
			<div style="flex-shrink: 0;">
				<ClaudeCodeInput
					onSend={handleCCSend}
					onStop={handleCCStop}
					onSteer={handleCCSteer}
					isRunning={ccRunning ?? false}
				/>
			</div>
		{:else if focused?.type === 'agent' && focusedAgent}
			<!-- Agent chat view -->
			<div style="flex: 1; min-height: 0;">
				<AgentChatView agent={focusedAgent} />
			</div>
			<ChatInput
				onSend={handleAgentSend}
				disabled={agentLoading}
				placeholder="Plan, @ for context, / for commands"
				modelName={agentModelName}
			/>
		{:else}
			<!-- Empty state -->
			<div style="flex: 1; display: flex; align-items: center; justify-content: center; color: var(--text-muted); font-size: 13px;">
				<div style="text-align: center;">
					<div style="font-size: 32px; margin-bottom: 8px; animation: agent-bob 4s ease-in-out infinite; color: var(--banana-yellow);">A</div>
					<div>Create a new conversation to get started.</div>
				</div>
			</div>
			<ChatInput
				onSend={handleAgentSend}
				placeholder="Type to start a new agent..."
			/>
		{/if}
	</div>
</div>

<svelte:head>
	<style>
		@keyframes agent-bob {
			0%, 100% { transform: translateY(0); }
			50% { transform: translateY(-6px); }
		}
	</style>
</svelte:head>
