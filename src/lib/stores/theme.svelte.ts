/** Theme store - drives all colors via reactive Svelte state for Tauri compatibility */

export interface ThemeColors {
	bgCanvas: string;
	bgPrimary: string;
	bgSurface: string;
	bgElevated: string;
	bgCard: string;
	bgColumn: string;
	bgColumnHover: string;

	borderDefault: string;
	borderSubtle: string;
	borderBright: string;
	borderGlow: string;

	textPrimary: string;
	textSecondary: string;
	textMuted: string;

	accentPrimary: string;
	accentHover: string;
	accentDim: string;
	accentGlow: string;
	accentGreen: string;
	accentRed: string;
	accentBlue: string;
	accentPurple: string;
	accentOrange: string;
	accentAmber: string;
	accentIndigo: string;

	gradientTitlebar: string;
	gradientStatusbar: string;
	gradientPanelMain: string;
	gradientPanelChat: string;
	gradientModal: string;

	shadowPanel: string;
	shadowSm: string;
	shadowCard: string;
	shadowCardHover: string;
	shadowTitlebar: string;
	shadowStatusbar: string;
	glowAccent: string;

	panelBorder: string;
	panelTopBorder: string;
	glassBg: string;
	glassBorder: string;
}

const darkColors: ThemeColors = {
	bgCanvas: '#111114',
	bgPrimary: '#16161a',
	bgSurface: '#1c1c21',
	bgElevated: '#232329',
	bgCard: 'rgba(28, 28, 33, 0.75)',
	bgColumn: 'rgba(255, 255, 255, 0.02)',
	bgColumnHover: 'rgba(255, 255, 255, 0.04)',

	borderDefault: '#26262d',
	borderSubtle: '#1e1e24',
	borderBright: '#35353e',
	borderGlow: 'rgba(99, 102, 241, 0.2)',

	textPrimary: '#e2e6ed',
	textSecondary: '#8f97a4',
	textMuted: '#5f6672',

	accentPrimary: '#6366f1',
	accentHover: '#818cf8',
	accentDim: '#4f46e5',
	accentGlow: 'rgba(99, 102, 241, 0.35)',
	accentGreen: '#3fb950',
	accentRed: '#f85149',
	accentBlue: '#58a6ff',
	accentPurple: '#bc8cff',
	accentOrange: '#d29922',
	accentAmber: '#f59e0b',
	accentIndigo: '#6366f1',

	gradientTitlebar: 'linear-gradient(180deg, #1a1a1f 0%, #16161a 100%)',
	gradientStatusbar: 'linear-gradient(0deg, #0e0e11 0%, #16161a 100%)',
	gradientPanelMain: 'linear-gradient(180deg, #1c1c21 0%, #16161a 100%)',
	gradientPanelChat: 'linear-gradient(180deg, #1c1c21 0%, #111114 100%)',
	gradientModal: 'linear-gradient(180deg, #232329 0%, #1c1c21 100%)',

	shadowPanel: '0 4px 20px rgba(0,0,0,0.35), 0 1px 4px rgba(0,0,0,0.2), 0 0 0 1px rgba(255,255,255,0.04)',
	shadowSm: '0 2px 6px rgba(0,0,0,0.2)',
	shadowCard: '0 4px 16px rgba(0,0,0,0.25), 0 0 0 1px rgba(99,102,241,0.06)',
	shadowCardHover: '0 8px 24px rgba(0,0,0,0.35), 0 0 0 1px rgba(99,102,241,0.15), 0 0 20px rgba(99,102,241,0.08)',
	shadowTitlebar: '0 2px 8px rgba(0,0,0,0.25), inset 0 1px 0 rgba(255,255,255,0.04)',
	shadowStatusbar: '0 -2px 8px rgba(0,0,0,0.2), inset 0 1px 0 rgba(255,255,255,0.03)',
	glowAccent: '0 0 16px rgba(99,102,241,0.15)',

	panelBorder: '1px solid rgba(255,255,255,0.06)',
	panelTopBorder: '1px solid rgba(99,102,241,0.08)',
	glassBg: 'rgba(24, 24, 30, 0.7)',
	glassBorder: 'rgba(255, 255, 255, 0.05)',
};

const lightColors: ThemeColors = {
	bgCanvas: '#eef0f4',
	bgPrimary: '#f5f6f9',
	bgSurface: '#ffffff',
	bgElevated: '#f8f9fb',
	bgCard: 'rgba(255, 255, 255, 0.9)',
	bgColumn: 'rgba(0, 0, 0, 0.02)',
	bgColumnHover: 'rgba(0, 0, 0, 0.04)',

	borderDefault: '#d0d5dd',
	borderSubtle: '#e2e5ec',
	borderBright: '#adb3bf',
	borderGlow: 'rgba(79, 70, 229, 0.15)',

	textPrimary: '#111318',
	textSecondary: '#4a5060',
	textMuted: '#7a8194',

	accentPrimary: '#4f46e5',
	accentHover: '#6366f1',
	accentDim: '#4338ca',
	accentGlow: 'rgba(79, 70, 229, 0.2)',
	accentGreen: '#16a34a',
	accentRed: '#dc2626',
	accentBlue: '#2563eb',
	accentPurple: '#7c3aed',
	accentOrange: '#ca8a04',
	accentAmber: '#d97706',
	accentIndigo: '#4f46e5',

	gradientTitlebar: 'linear-gradient(180deg, #ffffff 0%, #f7f8fa 100%)',
	gradientStatusbar: 'linear-gradient(0deg, #eceef2 0%, #f0f2f5 100%)',
	gradientPanelMain: 'linear-gradient(180deg, #ffffff 0%, #f7f8fa 100%)',
	gradientPanelChat: 'linear-gradient(180deg, #ffffff 0%, #f0f2f5 100%)',
	gradientModal: 'linear-gradient(180deg, #ffffff 0%, #f7f8fa 100%)',

	shadowPanel: '0 4px 20px rgba(0,0,0,0.06), 0 1px 4px rgba(0,0,0,0.04), 0 0 0 1px rgba(0,0,0,0.05)',
	shadowSm: '0 2px 6px rgba(0,0,0,0.06)',
	shadowCard: '0 4px 16px rgba(0,0,0,0.06), 0 0 0 1px rgba(79,70,229,0.06)',
	shadowCardHover: '0 8px 24px rgba(0,0,0,0.1), 0 0 0 1px rgba(79,70,229,0.12), 0 0 20px rgba(79,70,229,0.06)',
	shadowTitlebar: '0 2px 8px rgba(0,0,0,0.06), inset 0 -1px 0 rgba(0,0,0,0.05)',
	shadowStatusbar: '0 -2px 8px rgba(0,0,0,0.05), inset 0 1px 0 rgba(0,0,0,0.03)',
	glowAccent: '0 0 16px rgba(79,70,229,0.1)',

	panelBorder: '1px solid rgba(0,0,0,0.08)',
	panelTopBorder: '1px solid rgba(79,70,229,0.1)',
	glassBg: 'rgba(255, 255, 255, 0.7)',
	glassBorder: 'rgba(0, 0, 0, 0.06)',
};

export type ThemeMode = 'dark' | 'light';
const STORAGE_KEY = 'samwise_theme';

function detectInitialMode(): ThemeMode {
	if (typeof localStorage !== 'undefined') {
		const stored = localStorage.getItem(STORAGE_KEY) as ThemeMode | null;
		if (stored === 'dark' || stored === 'light') return stored;
	}
	if (typeof window !== 'undefined' && window.matchMedia?.('(prefers-color-scheme: light)').matches) {
		return 'light';
	}
	return 'dark';
}

const initialMode = detectInitialMode();
let mode = $state<ThemeMode>(initialMode);
let colors = $state<ThemeColors>(initialMode === 'light' ? lightColors : darkColors);

function applyToDOM(c: ThemeColors) {
	if (typeof document === 'undefined') return;
	document.documentElement.setAttribute('data-theme', mode);
	const s = document.documentElement.style;
	s.setProperty('--bg-canvas', c.bgCanvas);
	s.setProperty('--bg-primary', c.bgPrimary);
	s.setProperty('--bg-surface', c.bgSurface);
	s.setProperty('--bg-elevated', c.bgElevated);
	s.setProperty('--bg-card', c.bgCard);
	s.setProperty('--bg-column', c.bgColumn);
	s.setProperty('--bg-column-header-hover', c.bgColumnHover);
	s.setProperty('--border-default', c.borderDefault);
	s.setProperty('--border-subtle', c.borderSubtle);
	s.setProperty('--border-bright', c.borderBright);
	s.setProperty('--border-glow', c.borderGlow);
	s.setProperty('--text-primary', c.textPrimary);
	s.setProperty('--text-secondary', c.textSecondary);
	s.setProperty('--text-muted', c.textMuted);
	s.setProperty('--accent-primary', c.accentPrimary);
	s.setProperty('--accent-hover', c.accentHover);
	s.setProperty('--accent-dim', c.accentDim);
	s.setProperty('--accent-glow', c.accentGlow);
	s.setProperty('--accent-green', c.accentGreen);
	s.setProperty('--accent-red', c.accentRed);
	s.setProperty('--accent-blue', c.accentBlue);
	s.setProperty('--accent-purple', c.accentPurple);
	s.setProperty('--accent-orange', c.accentOrange);
	s.setProperty('--accent-amber', c.accentAmber);
	s.setProperty('--accent-indigo', c.accentIndigo);
	s.setProperty('--shadow-panel', c.shadowPanel);
	s.setProperty('--shadow-sm', c.shadowSm);
	s.setProperty('--shadow-card', c.shadowCard);
	s.setProperty('--shadow-card-hover', c.shadowCardHover);
	s.setProperty('--glow-accent', c.glowAccent);
	s.setProperty('--glass-bg', c.glassBg);
	s.setProperty('--glass-bg', c.glassBg);
	s.setProperty('--glass-border', c.glassBorder);
	s.setProperty('--glass-blur', '12px');
	s.setProperty('--panel-border', c.panelBorder);
	s.setProperty('--panel-top-border', c.panelTopBorder);
	s.setProperty('--gradient-modal', c.gradientModal);
	s.setProperty('--shadow-titlebar', c.shadowTitlebar);
	s.setProperty('--shadow-statusbar', c.shadowStatusbar);
	s.setProperty('--font-ui', "'Space Grotesk', system-ui, -apple-system, sans-serif");
	s.setProperty('--font-mono', "'JetBrains Mono', 'Cascadia Code', 'Fira Code', monospace");
	document.body.style.background = c.bgCanvas;
	document.body.style.color = c.textPrimary;
	document.body.style.fontFamily = "var(--font-ui)";
}

// Apply immediately on load
applyToDOM(colors);

function setMode(m: ThemeMode) {
	mode = m;
	colors = m === 'light' ? lightColors : darkColors;
	applyToDOM(colors);
	if (typeof localStorage !== 'undefined') {
		localStorage.setItem(STORAGE_KEY, m);
	}
}

export function getTheme() {
	return {
		get mode() { return mode; },
		get isDark() { return mode === 'dark'; },
		get c() { return colors; },
		toggle() { setMode(mode === 'dark' ? 'light' : 'dark'); },
		setMode,
		applyNow() { applyToDOM(colors); },
	};
}
