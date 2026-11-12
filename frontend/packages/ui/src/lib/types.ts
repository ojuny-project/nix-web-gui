export type WidgetKind =
  | "switch"
  | "dropdown"
  | "radio"
  | "spin"
  | "slider"
  | "text"
  | "multiline"
  | "path"
  | "package"
  | "packages"
  | "list"
  | "color"
  | "submodule"
  | "horizontal-submodule"
  | "table-attrset"
  | "table-list"
  | "font"
  | "icon-theme"
  | "cursor-theme"
  | "theme"
  | "wallpaper"
  | "file"
  | "icon"
  | "keybinds"
  | "monitors"
  | "menu"
  | "plugins"
  | "bar"
  | "dock"
  | "grid-configuration"
  | "users"
  | "facter"
  | "docker"
  | "secureboot"
  | "raw";

export type ControlDescriptor = {
  path: string;
  widget: WidgetKind;
  label: string;
  description?: string | null;
  choices?: string[] | null;
  min?: number | null;
  max?: number | null;
  step?: number | null;
  digits?: number | null;
  unit?: string | null;
  placeholder?: string | null;
  rows?: number | null;
  readonly?: boolean;
  advanced?: boolean;
  hidden?: boolean;
  group?: string | null;
  order?: number | null;
  groupOrder?: number | null;
};

export type ControlValue = unknown;
export type ChangeHandler = (path: string, value: ControlValue) => void;
