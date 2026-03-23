<script lang="ts">
	interface Props {
		busy?: boolean;
		size?: number;
	}

	let { busy = false, size = 28 }: Props = $props();
</script>

<div style="
	width: {size}px; height: {size}px; border-radius: 50%;
	border: 1.5px solid rgba(99, 102, 241, 0.3);
	display: flex; align-items: center; justify-content: center;
	flex-shrink: 0; position: relative; overflow: hidden;
	{busy ? 'animation: pulse-ring 2s ease-out infinite; box-shadow: 0 0 8px rgba(99,102,241,0.4);' : 'animation: robot-breathe 4s ease-in-out infinite;'}
">
	<img
		src="/samwise-avatar.png"
		alt="Sam"
		width={size}
		height={size}
		style="width: 100%; height: 100%; object-fit: cover; border-radius: 50%;"
	/>
</div>
