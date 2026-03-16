/** Layout store using Svelte 5 runes - three panel layout for Agent One */

let kanbanPanelWidth = $state(380);
let automationPanelWidth = $state(320);
let terminalHeight = $state(200);
let terminalVisible = $state(false);
let leftPanelVisible = $state(true);
let rightPanelVisible = $state(true);
let sidebarCollapsed = $state(false);
let doneColumnCollapsed = $state(false);
let focusedConversation = $state<{ id: string; type: 'agent' | 'claude-code' } | null>(null);

export function getLayout() {
	return {
		get kanbanPanelWidth() { return kanbanPanelWidth; },
		set kanbanPanelWidth(v: number) { kanbanPanelWidth = Math.max(300, Math.min(700, v)); },

		// Keep old name for compat
		get agentPanelWidth() { return kanbanPanelWidth; },
		set agentPanelWidth(v: number) { kanbanPanelWidth = Math.max(300, Math.min(700, v)); },

		get automationPanelWidth() { return automationPanelWidth; },
		set automationPanelWidth(v: number) { automationPanelWidth = Math.max(260, Math.min(500, v)); },

		// Keep old name for compat
		get filePanelWidth() { return automationPanelWidth; },
		set filePanelWidth(v: number) { automationPanelWidth = Math.max(260, Math.min(500, v)); },

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

		get doneColumnCollapsed() { return doneColumnCollapsed; },
		set doneColumnCollapsed(v: boolean) { doneColumnCollapsed = v; },
		toggleDoneColumn() { doneColumnCollapsed = !doneColumnCollapsed; },

		get focusedConversation() { return focusedConversation; },
		set focusedConversation(v: { id: string; type: 'agent' | 'claude-code' } | null) { focusedConversation = v; },

		get chatMode(): 'agent' | 'claude-code' {
			return focusedConversation?.type ?? 'agent';
		},

		toggleTerminal() { terminalVisible = !terminalVisible; },
		toggleLeftPanel() { leftPanelVisible = !leftPanelVisible; },
		toggleRightPanel() { rightPanelVisible = !rightPanelVisible; }
	};
}
