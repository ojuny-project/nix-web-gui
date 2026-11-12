export type User = {
  username: string;
  csrf: string;
};

export type ThemeState = {
  name: string;
  source: string;
  version: number;
};

async function readError(response: Response): Promise<string> {
  const body = await response.json().catch(() => ({}));
  return body.error ?? `${response.status} ${response.statusText}`;
}

export async function me(): Promise<User | null> {
  const response = await fetch("/api/me", { credentials: "same-origin" });
  if (response.status === 401) return null;
  if (!response.ok) throw new Error(await readError(response));
  return response.json();
}

export async function login(username: string, password: string): Promise<User> {
  const response = await fetch("/api/login", {
    method: "POST",
    credentials: "same-origin",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ username, password }),
  });
  if (!response.ok) throw new Error(await readError(response));
  return response.json();
}

export async function logout(): Promise<void> {
  await fetch("/api/logout", {
    method: "POST",
    credentials: "same-origin",
  });
}

export async function getTheme(): Promise<ThemeState> {
  const response = await fetch("/api/theme", { credentials: "same-origin" });
  if (!response.ok) throw new Error(await readError(response));
  return response.json();
}

export async function rpc<T>(
  method: string,
  params: unknown,
  csrf: string,
): Promise<T> {
  const response = await fetch("/api/rpc", {
    method: "POST",
    credentials: "same-origin",
    headers: {
      "content-type": "application/json",
      "x-csrf-token": csrf,
    },
    body: JSON.stringify({ jsonrpc: "2.0", id: 1, method, params }),
  });
  if (!response.ok) throw new Error(await readError(response));
  const body = await response.json();
  if (body.error) throw new Error(body.error.message);
  return body.result as T;
}

export type ScanRecord = {
  path: string;
  file: string;
  value: string;
  label?: string | null;
  description?: string | null;
  widget?: string | null;
  choices?: string[] | null;
  advanced?: boolean;
  hidden?: boolean;
  readonly?: boolean;
  group?: string | null;
  groupOrder?: number | null;
  order?: number | null;
  min?: number | null;
  max?: number | null;
  step?: number | null;
  digits?: number | null;
  unit?: string | null;
  rows?: number | null;
  placeholder?: string | null;
};

export type ScanMenu = {
  file: string;
  displayname: string;
  description?: string | null;
  category?: string | null;
  icon?: string | null;
  order: number;
  systems?: string[] | null;
  hidden?: boolean;
  records: ScanRecord[];
};

export type ScanResult = {
  menus: ScanMenu[];
};

export type PreviewEdit = {
  file: string;
  start: number;
  end: number;
  before: string | null;
  after: string;
};

export type PreviewResult = {
  edits: PreviewEdit[];
};

export type ServerState = {
  flake: string | null;
  auth: "pam" | "dev";
  theme: ThemeState;
};

export async function getState(): Promise<ServerState> {
  const response = await fetch("/api/state", { credentials: "same-origin" });
  if (!response.ok) throw new Error(await readError(response));
  return response.json();
}
