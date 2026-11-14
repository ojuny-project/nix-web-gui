<script lang="ts">
  import { onMount } from "svelte";
  import { me, type User } from "./api";
  import { watchTheme } from "./theme";
  import Login from "./pages/Login.svelte";
  import Shell from "./pages/Shell.svelte";

  let user: User | null = null;
  let loading = true;
  const popoutPage =
    new URLSearchParams(window.location.search).get("popout") ?? null;

  onMount(async () => {
    watchTheme().catch((error) => console.error("theme", error));
    try {
      user = await me();
    } catch (error) {
      console.error("session", error);
    } finally {
      loading = false;
    }
  });

  function onLogin(next: User) {
    user = next;
  }

  function onLogout() {
    user = null;
  }
</script>

{#if loading}
  <main class="boot">Loading...</main>
{:else if user}
  <Shell {user} {onLogout} popout={popoutPage} />
{:else}
  <Login {onLogin} />
{/if}
