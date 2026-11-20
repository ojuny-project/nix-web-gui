<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { Terminal } from "@xterm/xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import "@xterm/xterm/css/xterm.css";

  export let root: string;

  let host: HTMLDivElement | null = null;
  let command = "";
  let running = false;
  let status = "Enter a command to run in a terminal.";
  let terminal: Terminal | null = null;
  let fit: FitAddon | null = null;
  let socket: WebSocket | null = null;
  let observer: ResizeObserver | null = null;

  onMount(() => {
    terminal = new Terminal({
      convertEol: false,
      cursorBlink: true,
      fontSize: 13,
      fontFamily: '"JetBrains Mono", "Cascadia Code", monospace',
      scrollback: 5000,
      theme: {
        background: "#1e1e2e",
        foreground: "#cdd6f4",
        cursor: "#f5e0dc",
        selectionBackground: "#585b70",
        black: "#45475a",
        red: "#f38ba8",
        green: "#a6e3a1",
        yellow: "#f9e2af",
        blue: "#89b4fa",
        magenta: "#cba6f7",
        cyan: "#94e2d5",
        white: "#bac2de",
        brightBlack: "#585b70",
        brightRed: "#f38ba8",
        brightGreen: "#a6e3a1",
        brightYellow: "#f9e2af",
        brightBlue: "#89b4fa",
        brightMagenta: "#cba6f7",
        brightCyan: "#94e2d5",
        brightWhite: "#a6adc8",
      },
    });
    fit = new FitAddon();
    terminal.loadAddon(fit);
    if (host) {
      terminal.open(host);
      fit.fit();
    }
    (window as unknown as { __nwgTerminal?: Terminal }).__nwgTerminal = terminal;
    terminal.onData((data) => {
      if (socket?.readyState === WebSocket.OPEN) {
        socket.send(JSON.stringify({ type: "input", data }));
      }
    });
    terminal.onResize(({ cols, rows }) => {
      if (socket?.readyState === WebSocket.OPEN) {
        socket.send(JSON.stringify({ type: "resize", cols, rows }));
      }
    });
    observer = new ResizeObserver(() => {
      fit?.fit();
    });
    if (host) observer.observe(host);
    return () => observer?.disconnect();
  });

  onDestroy(() => {
    socket?.close();
    terminal?.dispose();
    observer?.disconnect();
  });

  function run() {
    if (!command.trim() || !terminal) return;
    socket?.close();
    terminal.reset();
    terminal.focus();
    running = true;
    status = `Running ${command.trim()}`;
    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    socket = new WebSocket(`${protocol}//${window.location.host}/api/terminal`);
    socket.binaryType = "arraybuffer";
    socket.onopen = () => {
      socket?.send(
        JSON.stringify({
          type: "command",
          command: command.trim(),
          root: root || null,
        }),
      );
      fit?.fit();
    };
    socket.onmessage = (event) => {
      if (typeof event.data === "string") {
        try {
          const message = JSON.parse(event.data);
          if (message.type === "error") {
            terminal?.write(`\r\n\x1b[31m${message.message}\x1b[0m\r\n`);
            status = message.message;
          }
        } catch {
          terminal?.write(event.data);
        }
        return;
      }
      terminal?.write(new Uint8Array(event.data));
    };
    socket.onerror = () => {
      status = "Terminal connection failed";
      running = false;
    };
    socket.onclose = () => {
      running = false;
      status = "Command finished";
      terminal?.write("\r\n\x1b[90m[command finished]\x1b[0m\r\n");
    };
  }

  function stop() {
    socket?.close();
    socket = null;
    running = false;
    status = "Stopped";
  }

  function clear() {
    terminal?.reset();
  }
</script>

<div class="terminal-page">
  <div class="terminal-toolbar">
    <input
      id="terminal-command"
      class="terminal-command"
      bind:value={command}
      placeholder="cfdisk /dev/nvme0n1"
      aria-label="Command to run"
      disabled={running}
      onkeydown={(event) => {
        if (event.key === "Enter") run();
      }}
    />
    {#if running}
      <button class="ghost" onclick={stop}>Stop</button>
    {:else}
      <button onclick={run} disabled={!command.trim()}>Run</button>
    {/if}
    <button class="ghost" onclick={clear} disabled={running}>Clear</button>
    <span class="muted small">{status}</span>
  </div>
  <p class="muted small">
    The command runs as your device account in {root || "the home directory"}.
    Use <code>sudo</code> when the command needs root, for example
    <code>sudo cfdisk /dev/nvme0n1</code>. This is a command terminal, not a
    persistent shell.
  </p>
  <div class="terminal-host" bind:this={host}></div>
</div>

<style>
  .terminal-page {
    display: grid;
    gap: 0.6rem;
    height: 100%;
    min-height: 0;
  }

  .terminal-toolbar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.5rem;
  }

  .terminal-command {
    flex: 1;
    min-width: 14rem;
    font-family: "JetBrains Mono", "Cascadia Code", monospace;
  }

  .terminal-host {
    min-height: 0;
    height: 65vh;
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 0.4rem;
    background: #1e1e2e;
    overflow: hidden;
  }

  .terminal-host :global(.xterm) {
    height: 100%;
  }
</style>
