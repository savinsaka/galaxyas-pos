<script lang="ts">
  import { currentUser } from "$lib/stores/auth";
  import Login from "$lib/components/Login.svelte";
  import Ribbon from "$lib/components/Ribbon.svelte";
  import TabBar from "$lib/components/TabBar.svelte";
  import Workspace from "$lib/components/Workspace.svelte";
</script>

{#if !$currentUser}
  <Login />
{:else}
  <div class="app">
    <Ribbon />
    <TabBar />
    <Workspace />
  </div>
{/if}
