<script lang="ts">
	import '../app.css';
	import '$lib/stores/theme.svelte';

	let { children } = $props();
</script>

<svelte:head>
	<meta name="color-scheme" content="dark" />
</svelte:head>

{@render children()}
