<script lang="ts">
	interface Props {
		onSend: (message: string) => void;
		onStop: () => void;
		onSteer: (message: string) => void;
		isRunning: boolean;
		disabled?: boolean;
	}

	let { onSend, onStop, onSteer, isRunning, disabled = false }: Props = $props();

	let inputValue = $state('');
	let sendHovered = $state(false);
	let stopHovered = $state(false);
	let textareaEl = $state<HTMLTextAreaElement | null>(null);

	function autoResize() {
		if (!textareaEl) return;
		textareaEl.style.height = 'auto';
		textareaEl.style.height = Math.min(textareaEl.scrollHeight, 200) + 'px';
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter' && !e.shiftKey) {
			e.preventDefault();
			send();
		}
	}

	function send() {
		const trimmed = inputValue.trim();
		if (!trimmed || disabled) return;

		if (isRunning) {
			// Steer: interrupt and redirect
			onSteer(trimmed);
		} else {
			onSend(trimmed);
		}
		inputValue = '';
		if (textareaEl) {
			textareaEl.style.height = 'auto';
		}
	}

	$effect(() => {
		if (inputValue !== undefined) autoResize();
	});
</script>

<div style="padding: 8px 10px; border-top: 1px solid var(--border-default);">
	<div
		style="
			display: flex; align-items: flex-end; gap: 6px;
			background: var(--bg-elevated); border: 1px solid var(--border-default);
			border-radius: 10px; padding: 6px 10px;
			transition: border-color 0.15s ease;
			{isRunning ? 'border-color: var(--accent-dim);' : ''}
			{disabled ? 'opacity: 0.6;' : ''}
		"
	>
		<textarea
			bind:this={textareaEl}
			bind:value={inputValue}
			placeholder={isRunning ? 'Type to steer Claude...' : 'Ask Claude Code to build something...'}
			rows={1}
			{disabled}
			style="
				flex: 1; background: none; border: none; outline: none;
				color: var(--text-primary); font-family: var(--font-ui);
				font-size: 13px; resize: none; min-height: 20px;
				max-height: 200px; line-height: 1.4;
			"
			oninput={autoResize}
			onkeydown={handleKeydown}
			onfocus={(e) => { const p = (e.currentTarget as HTMLElement).parentElement; if (p) p.style.borderColor = 'var(--accent-primary)'; }}
			onblur={(e) => { const p = (e.currentTarget as HTMLElement).parentElement; if (p) p.style.borderColor = isRunning ? 'var(--accent-dim)' : 'var(--border-default)'; }}
		></textarea>

		{#if isRunning}
			<!-- Stop button -->
			<button
				style="
					width: 28px; height: 28px; display: flex; align-items: center; justify-content: center;
					background: {stopHovered ? 'rgba(239, 68, 68, 0.25)' : 'rgba(239, 68, 68, 0.15)'};
					border: 1px solid rgba(239, 68, 68, 0.3);
					border-radius: 6px; cursor: pointer; flex-shrink: 0;
					transition: all 0.12s ease;
					transform: {stopHovered ? 'scale(1.1)' : 'scale(1)'};
				"
				onmouseenter={() => stopHovered = true}
				onmouseleave={() => stopHovered = false}
				onclick={() => onStop()}
				aria-label="Stop"
			>
				<svg width="10" height="10" viewBox="0 0 10 10">
					<rect x="0" y="0" width="10" height="10" rx="1" fill="rgb(239, 68, 68)" />
				</svg>
			</button>
		{/if}

		<!-- Send button -->
		<button
			style="
				width: 28px; height: 28px; display: flex; align-items: center; justify-content: center;
				background: {disabled ? 'var(--text-muted)' : sendHovered ? 'var(--accent-hover)' : 'var(--accent-primary)'};
				border: none; border-radius: 6px;
				cursor: {disabled ? 'not-allowed' : 'pointer'};
				flex-shrink: 0; transition: all 0.12s ease;
				transform: {sendHovered && !disabled ? 'scale(1.1)' : 'scale(1)'};
			"
			onmouseenter={() => sendHovered = true}
			onmouseleave={() => sendHovered = false}
			onclick={send}
			{disabled}
			aria-label={isRunning ? 'Steer' : 'Send'}
		>
			<svg width="14" height="14" viewBox="0 0 16 16" fill="#0D1117">
				<path d="M1.724 1.053a.5.5 0 0 1 .546-.065l13 6.5a.5.5 0 0 1 0 .894l-13 6.5a.5.5 0 0 1-.7-.58L3.39 8.5H8a.5.5 0 0 0 0-1H3.39L1.57 1.618a.5.5 0 0 1 .154-.565z"/>
			</svg>
		</button>
	</div>
	<div style="display: flex; justify-content: space-between; padding: 4px 4px 0; font-size: 10px; color: var(--text-muted);">
		<span>{isRunning ? 'Enter to steer, Stop to cancel' : 'Enter to send, Shift+Enter for new line'}</span>
		<span style="color: var(--accent-dim);">Claude Code CLI</span>
	</div>
</div>
