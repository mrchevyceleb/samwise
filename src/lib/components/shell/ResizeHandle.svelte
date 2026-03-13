<script lang="ts">
	type Props = {
		direction?: 'vertical' | 'horizontal';
		onResize: (delta: number) => void;
	};

	let { direction = 'vertical', onResize }: Props = $props();
	let dragging = $state(false);
	let hovered = $state(false);

	function onPointerDown(e: PointerEvent) {
		e.preventDefault();
		dragging = true;
		const target = e.currentTarget as HTMLElement;
		target.setPointerCapture(e.pointerId);
	}

	function onPointerMove(e: PointerEvent) {
		if (!dragging) return;
		const delta = direction === 'vertical' ? e.movementX : e.movementY;
		onResize(delta);
	}

	function onPointerUp() {
		dragging = false;
	}
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class="resize-handle"
	class:vertical={direction === 'vertical'}
	class:horizontal={direction === 'horizontal'}
	class:active={dragging}
	class:hovered={hovered}
	onpointerdown={onPointerDown}
	onpointermove={onPointerMove}
	onpointerup={onPointerUp}
	onpointerenter={() => hovered = true}
	onpointerleave={() => { hovered = false; dragging = false; }}
	role="separator"
	aria-orientation={direction}
>
	<div class="handle-indicator"></div>
</div>

<style>
	.resize-handle {
		position: relative;
		background: var(--border-default);
		transition: background 0.15s ease;
		flex-shrink: 0;
		z-index: 10;
	}
	.resize-handle.vertical {
		width: 3px;
		cursor: col-resize;
	}
	.resize-handle.horizontal {
		height: 3px;
		cursor: row-resize;
	}
	.resize-handle.hovered,
	.resize-handle.active {
		background: var(--banana-yellow);
	}
	.handle-indicator {
		position: absolute;
		border-radius: 2px;
		background: transparent;
		transition: background 0.15s ease;
	}
	.vertical .handle-indicator {
		top: 50%;
		left: -2px;
		width: 7px;
		height: 32px;
		transform: translateY(-50%);
	}
	.horizontal .handle-indicator {
		left: 50%;
		top: -2px;
		height: 7px;
		width: 32px;
		transform: translateX(-50%);
	}
	.resize-handle.hovered .handle-indicator,
	.resize-handle.active .handle-indicator {
		background: rgba(255, 214, 10, 0.3);
	}
</style>
