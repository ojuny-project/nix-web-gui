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

export const SCHEMES = data.schemes as Scheme[];
const byId = new Map(SCHEMES.map((scheme) => [scheme.id, scheme]));

export function schemeById(id: string): Scheme {
  return byId.get(id) ?? byId.get(DEFAULT_THEME_ID) ?? SCHEMES[0];
}

export type SchemeMeta = {
  scheme: Scheme;
  category: string | null;
  variant: string;
  label: string;
};

export type SchemeGroup = {
  category: string | null;
  label: string;
  metas: SchemeMeta[];
};

function prefixMatches(prefix: string, all: string[]): number {
  return all.filter(
    (other) => other === prefix || other.startsWith(`${prefix}-`),
  ).length;
}

function splitVariant(
  name: string,
  all: string[],
): [string | null, string] {
  const tokens = name.split("-");
  let boundary = 0;
  for (let index = 0; index < tokens.length; index += 1) {
    if (index > 0) boundary += 1;
    boundary += tokens[index].length;
    const prefix = name.slice(0, boundary);
    if (all.includes(prefix) && prefixMatches(prefix, all) > 1) {
      return [prefix, name.slice(prefix.length + 1)];
    }
  }
  let end = name.length;
  for (let index = tokens.length - 1; index >= 1; index -= 1) {
    end -= tokens[index].length + 1;
    const prefix = name.slice(0, end);
    if (prefixMatches(prefix, all) > 1) {
      return [prefix, name.slice(prefix.length + 1)];
    }
  }
  return [null, name];
}

function prettyCategory(category: string): string {
  return category
    .split("-")
    .map((word, index) =>
      index === 0 ? word.charAt(0).toUpperCase() + word.slice(1) : word,
    )
    .join(" ");
}

export function schemeGroups(): SchemeGroup[] {
  const names = SCHEMES.map((scheme) => scheme.id);
  const metas: SchemeMeta[] = SCHEMES.map((scheme) => {
    const [category, variant] = splitVariant(scheme.id, names);
    return {
      scheme,
      category,
      variant,
      label: category ? (variant || "default") : scheme.id,
    };
  });
  const categories = metas
    .map((meta) => meta.category)
    .filter((category): category is string => Boolean(category));
  for (const meta of metas) {
    if (!meta.category) continue;
    const parent = categories
      .filter((category) => meta.category?.startsWith(`${category}-`))
      .sort((left, right) => left.length - right.length)[0];
    if (parent) {
      meta.variant = meta.scheme.id.slice(parent.length + 1);
      meta.category = parent;
      meta.label = meta.variant || "default";
    }
  }
  const groups: SchemeGroup[] = [];
  for (const meta of metas) {
    const last = groups.at(-1);
    if (!last || last.category !== meta.category) {
      groups.push({
        category: meta.category,
        label: meta.category ? prettyCategory(meta.category) : "",
        metas: [meta],
      });
    } else {
      last.metas.push(meta);
    }
  }
  return groups;
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
