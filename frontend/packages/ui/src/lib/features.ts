export const OJUNY_WIDGETS = import.meta.env.VITE_OJUNY_WIDGETS !== "0";

export const OJUNY_WIDGET_KINDS = new Set([
  "menu",
  "plugins",
  "bar",
  "dock",
  "grid-configuration",
  "docker",
]);
