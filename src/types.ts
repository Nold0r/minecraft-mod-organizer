export type Loader = "Fabric" | "Quilt" | "Forge" | "NeoForge" | "Unknown";

export interface Category {
  id: string;
  name: string;
  parentId: string | null;
  sortIndex: number;
  color: string | null;
  iconHex: string | null;
}

export interface PackagedMod {
  modId: string;
  name: string;
  version: string;
  loader: Loader;
}

export interface ModEntry {
  id: string;
  layoutKey: string;
  modId: string;
  name: string;
  version: string;
  loader: Loader;
  enabled: boolean;
  fileName: string;
  path: string;
  iconDataUrl: string | null;
  packagedMods: PackagedMod[];
  categoryId: string | null;
  sortIndex: number;
}

export interface WorkspaceSnapshot {
  modsDir: string | null;
  categories: Category[];
  mods: ModEntry[];
}

export type DragItem =
  | { kind: "category"; id: string }
  | { kind: "mod"; id: string };

export type DropTarget =
  | { kind: "root" }
  | { kind: "category-inside"; categoryId: string }
  | { kind: "category-before"; categoryId: string }
  | { kind: "category-after"; categoryId: string }
  | { kind: "mod-before"; modId: string }
  | { kind: "mod-after"; modId: string };
