import data from "./data/base16.json";

export type SchemeVariant = "dark" | "light";

export type Scheme = {
  id: string;
  name: string;
  author: string;
  variant: SchemeVariant;
  palette: Record<string, string>;
};

export const THEME_EVENT = "nix-web-gui:theme-change";
export const DEFAULT_THEME_ID: string = data.default;

const schemes = data.schemes as Scheme[];
const byId = new Map(schemes.map((scheme) => [scheme.id, scheme]));

export function schemeById(id: string): Scheme {
  return byId.get(id) ?? byId.get(DEFAULT_THEME_ID) ?? schemes[0];
}

export function applyTheme(id: string): Scheme {
  const scheme = schemeById(id);
  const root = document.documentElement;

  for (const [key, value] of Object.entries(scheme.palette)) {
    root.style.setProperty(`--${key.toLowerCase()}`, value);
  }

  root.dataset.theme = scheme.variant;
  root.dataset.scheme = scheme.id;
  root.style.colorScheme = scheme.variant;

  const themeColor = document.querySelector<HTMLMetaElement>(
    'meta[name="theme-color"]',
  );
  if (themeColor) {
    themeColor.content = scheme.palette.base00;
  }

  window.dispatchEvent(new CustomEvent(THEME_EVENT, { detail: scheme }));
  return scheme;
}
