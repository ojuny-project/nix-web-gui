<script lang="ts">
  import { login, type User } from "../api";

  export let onLogin: (user: User) => void;

  let username = "";
  let password = "";
  let error = "";
  let busy = false;

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    error = "";
    busy = true;
    try {
      onLogin(await login(username, password));
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Login failed";
    } finally {
      busy = false;
    }
  }
</script>

<main class="login">
  <section class="login-card">
    <p class="eyebrow">nix-web-gui</p>
    <h1>Sign in</h1>
    <p class="muted">
      Use your device account. The same account works here and on the local
      settings window.
    </p>

    <form onsubmit={submit}>
      <label>
        <span>Username</span>
        <input
          bind:value={username}
          name="username"
          autocomplete="username"
          autocapitalize="none"
          required
        />
      </label>

      <label>
        <span>Password</span>
        <input
          bind:value={password}
          name="password"
          type="password"
          autocomplete="current-password"
          required
        />
      </label>

      {#if error}
        <p class="error" role="alert">{error}</p>
      {/if}

      <button type="submit" disabled={busy}>
        {busy ? "Signing in..." : "Sign in"}
      </button>
    </form>
  </section>
</main>
