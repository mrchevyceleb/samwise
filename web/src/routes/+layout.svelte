<script lang="ts">
  import '../app.css';
  import FloatingBg from '$lib/components/FloatingBg.svelte';
  import ClickEgg from '$lib/components/ClickEgg.svelte';
  let { children } = $props();
</script>

<FloatingBg />
<ClickEgg />
{@render children()}
