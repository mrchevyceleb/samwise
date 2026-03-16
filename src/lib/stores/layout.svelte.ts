/** Layout store using Svelte 5 runes (module-level reactive state) */

let agentPanelWidth = $state(480);
let filePanelWidth = $state(280);
let terminalHeight = $state(200);
let terminalVisible = $state(false);
let leftPanelVisible = $state(true);
let rightPanelVisible = $state(true);
let sidebarCollapsed = $state(false);
let focusedConversation = $state<{ id: string; type: 'agent' | 'claude-code' } | null>(null);

export function getLayout() {
	return {
		get agentPanelWidth() { return agentPanelWidth; },
		set agentPanelWidth(v: number) { agentPanelWidth = Math.max(520, Math.min(900, v)); },

		get filePanelWidth() { return filePanelWidth; },
		set filePanelWidth(v: number) { filePanelWidth = Math.max(200, Math.min(500, v)); },

		get terminalHeight() { return terminalHeight; },
		set terminalHeight(v: number) { terminalHeight = Math.max(100, Math.min(500, v)); },

		get terminalVisible() { return terminalVisible; },
		set terminalVisible(v: boolean) { terminalVisible = v; },

		get leftPanelVisible() { return leftPanelVisible; },
		set leftPanelVisible(v: boolean) { leftPanelVisible = v; },

		get rightPanelVisible() { return rightPanelVisible; },
		set rightPanelVisible(v: boolean) { rightPanelVisible = v; },

		get sidebarCollapsed() { return sidebarCollapsed; },
		set sidebarCollapsed(v: boolean) { sidebarCollapsed = v; },
		toggleSidebar() { sidebarCollapsed = !sidebarCollapsed; },

		get focusedConversation() { return focusedConversation; },
		set focusedConversation(v: { id: string; type: 'agent' | 'claude-code' } | null) { focusedConversation = v; },

		// Backwards compat helper
		get chatMode(): 'agent' | 'claude-code' {
			return focusedConversation?.type ?? 'agent';
		},

		toggleTerminal() { terminalVisible = !terminalVisible; },
		toggleLeftPanel() { leftPanelVisible = !leftPanelVisible; },
		toggleRightPanel() { rightPanelVisible = !rightPanelVisible; }
	};
}
