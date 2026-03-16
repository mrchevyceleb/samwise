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

	let lottieContainer = $state<HTMLDivElement | null>(null);
	let lottieAnim: any = null;

	$effect(() => {
		if (!lottieContainer || lottieAnim) return;
		// Load lottie-web and start animation
		import('lottie-web').then((lottie) => {
			lottieAnim = lottie.default.loadAnimation({
				container: lottieContainer!,
				renderer: 'svg',
				loop: true,
				autoplay: true,
				path: '/banana-anim.json'
			});
		});
		return () => {
			if (lottieAnim) {
				lottieAnim.destroy();
				lottieAnim = null;
			}
		};
	});

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
			<div style="flex: 1; display: flex; align-items: center; justify-content: center; color: var(--text-muted); font-size: 13px; position: relative; overflow: hidden;">
				<!-- Radial glow -->
				<div style="position: absolute; width: 300px; height: 300px; border-radius: 50%; background: radial-gradient(circle, color-mix(in srgb, var(--accent-primary) 5%, transparent) 0%, transparent 70%); pointer-events: none;"></div>
				<div style="text-align: center; position: relative; z-index: 1;">
					<!-- Banana mascot lottie -->
					<div
						bind:this={lottieContainer}
						style="width: 140px; height: 140px; margin: 0 auto 4px; filter: drop-shadow(0 0 20px color-mix(in srgb, var(--accent-primary) 15%, transparent));"
					></div>
					<div style="font-size: 14px; font-weight: 700; color: var(--text-primary); margin-bottom: 4px; letter-spacing: -0.3px;">Ready to vibe</div>
					<div style="color: var(--text-muted); font-size: 12px; line-height: 1.5;">Start a conversation or just<br/>type below to get going.</div>
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
