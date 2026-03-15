/** Layout store using Svelte 5 runes (module-level reactive state) */

let agentPanelWidth = $state(350);
let filePanelWidth = $state(280);
let terminalHeight = $state(200);
let terminalVisible = $state(false);
let chatMode = $state<'agent' | 'claude-code'>('agent');

export function getLayout() {
	return {
		get agentPanelWidth() { return agentPanelWidth; },
		set agentPanelWidth(v: number) { agentPanelWidth = Math.max(280, Math.min(600, v)); },

		get filePanelWidth() { return filePanelWidth; },
		set filePanelWidth(v: number) { filePanelWidth = Math.max(200, Math.min(500, v)); },

		get terminalHeight() { return terminalHeight; },
		set terminalHeight(v: number) { terminalHeight = Math.max(100, Math.min(500, v)); },

		get terminalVisible() { return terminalVisible; },
		set terminalVisible(v: boolean) { terminalVisible = v; },

		get chatMode() { return chatMode; },
		set chatMode(v: 'agent' | 'claude-code') { chatMode = v; },

		toggleTerminal() { terminalVisible = !terminalVisible; }
	};
}
