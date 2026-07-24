<script lang="ts">
  import "../app.css";

  let { children } = $props();
</script>

<div class="shell">
  {@render children()}
</div>

<style>
  .shell {
    height: 100%;
    min-height: 100%;
  }
</style>
