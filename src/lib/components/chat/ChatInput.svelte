<script lang="ts">
	interface Props {
		disabled?: boolean;
		placeholder?: string;
		onSend: (message: string) => void;
		modelName?: string;
	}

	let { disabled = false, placeholder = 'Talk to your AI employee...', onSend, modelName = '' }: Props = $props();

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
		if (textareaEl) textareaEl.style.height = 'auto';
	}

	function handleQuickAction(action: string) {
		if (action === 'create-task') {
			onSend('/create-task');
		} else if (action === 'run-qa') {
			onSend('/run-qa');
		} else if (action === 'check-status') {
			onSend('/status');
		}
	}

	$effect(() => {
		if (inputValue !== undefined) autoResize();
	});
</script>

<div style="padding: 10px 12px; position: relative; background: linear-gradient(0deg, rgba(13, 17, 23, 0.8) 0%, transparent 100%);">
	<div
		style="
			display: flex; flex-direction: column; gap: 0;
			background: linear-gradient(180deg, rgba(28, 35, 51, 0.9) 0%, rgba(22, 27, 34, 0.95) 100%);
			border: 1px solid rgba(99, 102, 241, 0.1);
			border-radius: 14px; overflow: hidden;
			transition: border-color 0.2s ease, box-shadow 0.2s ease;
			box-shadow: 0 4px 16px rgba(0, 0, 0, 0.3), inset 0 1px 0 rgba(255, 255, 255, 0.03);
			{disabled ? 'opacity: 0.6;' : ''}
		"
	>
		<!-- Textarea -->
		<div style="padding: 8px 10px 4px;">
			<textarea
				bind:this={textareaEl}
				bind:value={inputValue}
				{placeholder}
				rows={2}
				{disabled}
				style="
					width: 100%; background: none; border: none; outline: none;
					color: var(--text-primary); font-family: var(--font-ui);
					font-size: 13px; resize: none; min-height: 36px;
					max-height: 200px; line-height: 1.5;
				"
				oninput={autoResize}
				onkeydown={handleKeydown}
				onfocus={(e) => {
					const p = (e.currentTarget as HTMLElement).closest('[style*="border-radius: 14px"]') as HTMLElement | null;
					if (p) {
						p.style.borderColor = 'rgba(99, 102, 241, 0.3)';
						p.style.boxShadow = '0 4px 16px rgba(0, 0, 0, 0.3), 0 0 0 1px rgba(99, 102, 241, 0.1), 0 0 20px rgba(99, 102, 241, 0.05)';
					}
				}}
				onblur={(e) => {
					const p = (e.currentTarget as HTMLElement).closest('[style*="border-radius: 14px"]') as HTMLElement | null;
					if (p) {
						p.style.borderColor = 'rgba(99, 102, 241, 0.1)';
						p.style.boxShadow = '0 4px 16px rgba(0, 0, 0, 0.3), inset 0 1px 0 rgba(255, 255, 255, 0.03)';
					}
				}}
			></textarea>
		</div>

		<!-- Toolbar row -->
		<div style="display: flex; align-items: center; gap: 4px; padding: 4px 8px 6px;">
			<div style="flex: 1;"></div>

			<!-- Model pill -->
			{#if modelName}
				<span style="
					padding: 2px 8px; border-radius: 10px; font-size: 10px;
					background: rgba(99, 102, 241, 0.08); color: var(--text-muted);
					font-family: var(--font-mono); white-space: nowrap;
				">
					{modelName}
				</span>
			{/if}

			<!-- Send button -->
			<button
				style="
					width: 30px; height: 30px; display: flex; align-items: center;
					justify-content: center;
					background: {disabled ? 'var(--text-muted)' : sendHovered ? 'var(--accent-hover)' : 'var(--accent-primary)'};
					border: none; border-radius: 10px;
					cursor: {disabled ? 'not-allowed' : 'pointer'};
					flex-shrink: 0; transition: all 0.15s ease;
					transform: {sendHovered && !disabled ? 'scale(1.1) rotate(-5deg)' : 'scale(1)'};
					box-shadow: {disabled ? 'none' : sendHovered ? '0 4px 16px rgba(99, 102, 241, 0.35)' : '0 2px 8px rgba(99, 102, 241, 0.2)'};
				"
				onmouseenter={() => sendHovered = true}
				onmouseleave={() => sendHovered = false}
				onclick={send}
				{disabled}
				aria-label="Send message"
			>
				<svg width="14" height="14" viewBox="0 0 16 16" fill="white">
					<path d="M1.724 1.053a.5.5 0 0 1 .546-.065l13 6.5a.5.5 0 0 1 0 .894l-13 6.5a.5.5 0 0 1-.7-.58L3.39 8.5H8a.5.5 0 0 0 0-1H3.39L1.57 1.618a.5.5 0 0 1 .154-.565z"/>
				</svg>
			</button>
		</div>
	</div>

	<div style="display: flex; justify-content: space-between; padding: 4px 4px 0; font-size: 10px; color: var(--text-muted);">
		<span>Enter to send, Shift+Enter for new line</span>
	</div>
</div>
