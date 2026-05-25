/** Layout store using Svelte 5 runes - three panel layout for Agent One */

let kanbanPanelWidth = $state(380);
let automationPanelWidth = $state(320);
let terminalHeight = $state(200);
let terminalVisible = $state(false);
let leftPanelVisible = $state(true);
let rightPanelVisible = $state(true);
let sidebarCollapsed = $state(false);
const storedDoneColumnCollapsed = typeof localStorage !== 'undefined'
	? localStorage.getItem('agent-one-done-column-collapsed') === 'true'
	: false;
const storedFailedColumnCollapsed = typeof localStorage !== 'undefined'
	? localStorage.getItem('agent-one-failed-column-collapsed') === 'true'
	: false;
let doneColumnCollapsed = $state(storedDoneColumnCollapsed);
let failedColumnCollapsed = $state(storedFailedColumnCollapsed);
let focusedConversation = $state<{ id: string; type: 'agent' | 'claude-code' } | null>(null);

// Theme: 'dark' | 'light', persisted to localStorage
type Theme = 'dark' | 'light';
const storedTheme = typeof localStorage !== 'undefined' ? localStorage.getItem('agent-one-theme') as Theme | null : null;
let theme = $state<Theme>(storedTheme ?? 'dark');

function applyTheme(t: Theme) {
	if (typeof document !== 'undefined') {
		document.documentElement.classList.toggle('light', t === 'light');
	}
	if (typeof localStorage !== 'undefined') {
		localStorage.setItem('agent-one-theme', t);
	}
}

function persistBool(key: string, value: boolean) {
	if (typeof localStorage !== 'undefined') {
		localStorage.setItem(key, String(value));
	}
}

// Apply on load
if (typeof document !== 'undefined') {
	applyTheme(theme);
}

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
		set doneColumnCollapsed(v: boolean) {
			doneColumnCollapsed = v;
			persistBool('agent-one-done-column-collapsed', v);
		},
		toggleDoneColumn() {
			doneColumnCollapsed = !doneColumnCollapsed;
			persistBool('agent-one-done-column-collapsed', doneColumnCollapsed);
		},

		get failedColumnCollapsed() { return failedColumnCollapsed; },
		set failedColumnCollapsed(v: boolean) {
			failedColumnCollapsed = v;
			persistBool('agent-one-failed-column-collapsed', v);
		},
		toggleFailedColumn() {
			failedColumnCollapsed = !failedColumnCollapsed;
			persistBool('agent-one-failed-column-collapsed', failedColumnCollapsed);
		},

		get focusedConversation() { return focusedConversation; },
		set focusedConversation(v: { id: string; type: 'agent' | 'claude-code' } | null) { focusedConversation = v; },

		get chatMode(): 'agent' | 'claude-code' {
			return focusedConversation?.type ?? 'agent';
		},

		get theme() { return theme; },
		set theme(v: Theme) { theme = v; applyTheme(v); },
		toggleTheme() { const next = theme === 'dark' ? 'light' : 'dark'; theme = next; applyTheme(next); },
		get isDark() { return theme === 'dark'; },

		toggleTerminal() { terminalVisible = !terminalVisible; },
		toggleLeftPanel() { leftPanelVisible = !leftPanelVisible; },
		toggleRightPanel() { rightPanelVisible = !rightPanelVisible; }
	};
}
