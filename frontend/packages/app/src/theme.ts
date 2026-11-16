import { applyTheme } from "@nix-web-gui/ui";
import { getTheme } from "./api";

export async function watchTheme(): Promise<EventSource> {
  const current = await getTheme();
  applyTheme(current.name);

  const source = new EventSource("/api/events");
  const update = (event: MessageEvent) => {
    try {
      const state = JSON.parse(event.data) as { name: string };
      applyTheme(state.name);
    } catch {
      // Keep the last applied theme on a malformed event.
    }
  };

  source.addEventListener("theme", update);
  source.addEventListener("theme.changed", update);
  source.addEventListener("progress", (event: MessageEvent) => {
    try {
      const payload = JSON.parse(event.data);
      window.dispatchEvent(
        new CustomEvent("nix-web-gui:progress", { detail: payload }),
      );
    } catch {
      // Ignore a malformed progress note.
    }
  });
  return source;
}
