<script lang="ts">
	import '../app.css';

	let { children } = $props();
</script>

<svelte:head>
	<meta name="color-scheme" content="dark" />
</svelte:head>

{@render children()}
