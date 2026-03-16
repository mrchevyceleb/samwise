<script lang="ts">
	interface Props {
		busy?: boolean;
		size?: number;
	}

	let { busy = false, size = 28 }: Props = $props();
</script>

<div style="
	width: {size}px; height: {size}px; border-radius: {size * 0.3}px;
	background: linear-gradient(135deg, rgba(99, 102, 241, 0.2), rgba(129, 140, 248, 0.1));
	border: 1px solid rgba(99, 102, 241, 0.2);
	display: flex; align-items: center; justify-content: center;
	flex-shrink: 0; position: relative;
	{busy ? 'animation: pulse-ring 2s ease-out infinite;' : 'animation: robot-breathe 4s ease-in-out infinite;'}
">
	<!-- Robot face -->
	<svg width={size * 0.6} height={size * 0.6} viewBox="0 0 24 24" fill="none" stroke="var(--accent-indigo)" stroke-width="2" stroke-linecap="round">
		<rect x="4" y="8" width="16" height="12" rx="2"/>
		<!-- Antenna -->
		<line x1="12" y1="4" x2="12" y2="8"/>
		<circle cx="12" cy="3" r="1.5" fill="var(--accent-indigo)" style="{busy ? 'animation: pulse-dot 1s ease-in-out infinite;' : ''}"/>
		<!-- Eyes -->
		<circle cx="9" cy="14" r="1.5" fill="var(--accent-indigo)" style="animation: robot-blink 4s ease-in-out infinite;"/>
		<circle cx="15" cy="14" r="1.5" fill="var(--accent-indigo)" style="animation: robot-blink 4s ease-in-out infinite; animation-delay: 0.1s;"/>
	</svg>
</div>
