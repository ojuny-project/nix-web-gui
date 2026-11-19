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

export type Visibility = {
  path: string;
  equals?: string | null;
  equalsAny?: string[];
  notEquals?: string | null;
};

export type ScanRecord = {
  path: string;
  file: string;
  value: string;
  label?: string | null;
  description?: string | null;
  widget?: string | null;
  choices?: string[] | null;
  list?: string[] | null;
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
  type?: string | null;
  default?: string | null;
  applyCommand?: string | null;
  origin?: string | null;
  visibleWhen?: Visibility | null;
  children?: ScanRecord[] | null;
  depth?: number | null;
  monospace?: boolean;
  variant?: string | null;
  table?: TableJson | null;
};

export type ScanInsert = {
  name: string;
  group?: string | null;
  label?: string | null;
  offset: number;
  line: number;
};

export type ScanSnippetPlaceholder = {
  name: string;
  label: string;
  default: string;
  hint: string;
};

export type ScanSnippet = {
  name: string;
  label: string;
  placeholders: ScanSnippetPlaceholder[];
};

export type FileTemplate = {
  index: number;
  label: string;
  kind: string;
  filename: string;
  output: string;
  placeholders: ScanSnippetPlaceholder[];
};

export type TableRowJson = {
  key: string;
  label?: string | null;
  hidden?: boolean;
  fields: Record<string, string>;
};

export type TableJson = {
  list: boolean;
  columns: string[];
  columnTypes?: string[] | null;
  keyEditable: boolean;
  firstValueEditable: boolean;
  rows: TableRowJson[];
};

export type ScanMenu = {
  file: string;
  displayname: string;
  origin?: string | null;
  description?: string | null;
  category?: string | null;
  icon?: string | null;
  order: number;
  systems?: string[] | null;
  hidden?: boolean;
  records: ScanRecord[];
  inserts?: ScanInsert[];
  snippets?: ScanSnippet[];
};

export type ScanResult = {
  menus: ScanMenu[];
  hosts?: string[];
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
  configFlake?: string | null;
  recentFlakes?: string[];
  hostname?: string | null;
  page: string | null;
  auth: "pam" | "dev";
  theme: ThemeState;
};

export async function getState(): Promise<ServerState> {
  const response = await fetch("/api/state", { credentials: "same-origin" });
  if (!response.ok) throw new Error(await readError(response));
  return response.json();
}

export type FlakeInputJson = {
  name: string;
  url: string | null;
  flake: boolean;
  submodules: boolean;
  lfs: boolean;
  follows: Record<string, string>;
  description: string | null;
};

export type LockedInputJson = {
  type: string;
  owner: string | null;
  repo: string | null;
  ref: string | null;
  rev: string | null;
  lastModified: number | null;
  narHash: string | null;
};

export type FlakeInputsResult = {
  inputs: FlakeInputJson[];
  nixpkgs: FlakeInputJson | null;
  locked: LockedInputJson | null;
  missing?: Array<{
    name: string;
    url: string;
    file: string;
    line: number;
  }>;
};

export type InputCheck = {
  name: string;
  state?: string;
  reason?: string | null;
  currentRev?: string | null;
  currentDate?: string | null;
  remoteRev?: string | null;
  remoteDate?: string | null;
  error?: string;
};

export type GitStatus = {
  branch: string;
  upstream: string | null;
  dirty: boolean;
  ahead: number;
  behind: number;
  state: string;
};

export type WebConfig = {
  flake_path: string | null;
  terminal_command: string | null;
  rebuild_command: string | null;
  recent_flake_paths: string[];
  live_apply: boolean;
  reload_mango_on_rebuild: boolean;
  reload_dms_on_rebuild: boolean;
  expand_details_by_default: boolean;
  auto_git_sync: boolean;
};

export type StyleEntry = {
  label: string;
  value: string;
  package: string;
  monospace: boolean;
  variant: string;
  preview?: string | null;
  previews?: string[];
};

export type SecureBootState = {
  efi: boolean;
  firmware_mode: string;
  firmware: string | null;
  boot_into_firmware: boolean;
  sbctl_installed: boolean;
  owner_guid: string | null;
  setup_mode_enabled: boolean | null;
  vendor_keys: string[];
  esp_path: string | null;
  esp_free_bytes: number | null;
  key_bundle: string;
  keys_present: boolean;
  keys_enrolled: boolean | null;
  lanzaboote_active: boolean;
  signed_files: number;
  unsigned_files: number;
  windows_present: boolean;
  bitlocker_present: boolean;
  errors: string[];
};

export type SecureBootStatus = {
  state: SecureBootState;
  stage: { title: string; detail: string };
};

export type PreviewFile = {
  name: string;
  value: string;
  path: string;
};

export type PreviewPreset = {
  name: string;
  label: string;
  path: string;
  preview?: { mime: string; data: string } | null;
};
