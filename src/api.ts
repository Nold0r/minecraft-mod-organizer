import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { WorkspaceSnapshot } from "./types";

export const backend = {
  getSnapshot: () => invoke<WorkspaceSnapshot>("get_snapshot"),
  setModsDirectory: (path: string) =>
    invoke<WorkspaceSnapshot>("set_mods_directory", { path }),
  createCategory: (name: string, parentId: string | null) =>
    invoke<WorkspaceSnapshot>("create_category", { name, parentId }),
  deleteCategory: (id: string) =>
    invoke<WorkspaceSnapshot>("delete_category", { id }),
  setCategoryColor: (id: string, color: string | null) =>
    invoke<WorkspaceSnapshot>("set_category_color", { id, color }),
  setCategoryIcon: (id: string, iconHex: string | null) =>
    invoke<WorkspaceSnapshot>("set_category_icon", { id, iconHex }),
  moveCategory: (id: string, parentId: string | null, index: number) =>
    invoke<WorkspaceSnapshot>("move_category", { id, parentId, index }),
  moveMod: (id: string, categoryId: string | null, index: number) =>
    invoke<WorkspaceSnapshot>("move_mod", { id, categoryId, index }),
  setModEnabled: (id: string, enabled: boolean) =>
    invoke<WorkspaceSnapshot>("set_mod_enabled", { id, enabled }),
  rescan: () => invoke<WorkspaceSnapshot>("rescan"),
  onSnapshotChanged: (handler: (snapshot: WorkspaceSnapshot) => void) =>
    listen<WorkspaceSnapshot>("mods://changed", (event) => handler(event.payload)),
};
