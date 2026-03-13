<script lang="ts">
	interface Props {
		disabled?: boolean;
		placeholder?: string;
		onSend: (message: string) => void;
	}

	let { disabled = false, placeholder = 'Ask the agent to build something...', onSend }: Props = $props();

	let inputValue = $state('');
	let sendHovered = $state(false);
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
		onSend(trimmed);
		inputValue = '';
		if (textareaEl) {
			textareaEl.style.height = 'auto';
		}
	}

	$effect(() => {
		// Trigger auto-resize when value changes
		if (inputValue !== undefined) {
			autoResize();
		}
	});
</script>

<div style="padding: 8px 10px; border-top: 1px solid var(--border-default);">
	<div
		style="
			display: flex; align-items: flex-end; gap: 6px;
			background: var(--bg-elevated); border: 1px solid var(--border-default);
			border-radius: 10px; padding: 6px 10px;
			transition: border-color 0.15s ease;
			{disabled ? 'opacity: 0.6;' : ''}
		"
	>
		<textarea
			bind:this={textareaEl}
			bind:value={inputValue}
			{placeholder}
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
			onfocus={(e) => { const p = (e.currentTarget as HTMLElement).parentElement; if (p) p.style.borderColor = 'var(--banana-yellow)'; }}
			onblur={(e) => { const p = (e.currentTarget as HTMLElement).parentElement; if (p) p.style.borderColor = 'var(--border-default)'; }}
		></textarea>
		<button
			style="
				width: 28px; height: 28px; display: flex; align-items: center;
				justify-content: center;
				background: {disabled ? 'var(--text-muted)' : sendHovered ? 'var(--banana-yellow-hover)' : 'var(--banana-yellow)'};
				border: none; border-radius: 6px;
				cursor: {disabled ? 'not-allowed' : 'pointer'};
				flex-shrink: 0; transition: all 0.12s ease;
				transform: {sendHovered && !disabled ? 'scale(1.1)' : 'scale(1)'};
			"
			onmouseenter={() => sendHovered = true}
			onmouseleave={() => sendHovered = false}
			onclick={send}
			{disabled}
			aria-label="Send message"
		>
			<svg width="14" height="14" viewBox="0 0 16 16" fill="#0D1117">
				<path d="M1.724 1.053a.5.5 0 0 1 .546-.065l13 6.5a.5.5 0 0 1 0 .894l-13 6.5a.5.5 0 0 1-.7-.58L3.39 8.5H8a.5.5 0 0 0 0-1H3.39L1.57 1.618a.5.5 0 0 1 .154-.565z"/>
			</svg>
		</button>
	</div>
	<div style="display: flex; justify-content: space-between; padding: 4px 4px 0; font-size: 10px; color: var(--text-muted);">
		<span>Enter to send, Shift+Enter for new line</span>
		<span style="color: var(--banana-yellow-dim);">Free tier</span>
	</div>
</div>
