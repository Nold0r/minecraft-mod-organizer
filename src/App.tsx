import { DndContext, DragEndEvent, PointerSensor, useSensor, useSensors } from "@dnd-kit/core";
import { open } from "@tauri-apps/plugin-dialog";
import { useEffect, useMemo, useState } from "react";
import { backend } from "./api";
import { CategoryNode } from "./components/CategoryNode";
import { DropZone } from "./components/DropZone";
import { ModRow } from "./components/ModRow";
import { nerdGlyph, nerdIconHex } from "./icons";
import type { Category, DragItem, DropTarget, ModEntry, WorkspaceSnapshot } from "./types";
import "./styles.css";

const emptySnapshot: WorkspaceSnapshot = { modsDir: null, categories: [], mods: [] };

function insertIndexBeforeCategory(categories: Category[], targetId: string) {
  const target = categories.find((c) => c.id === targetId);
  if (!target) return { parentId: null, index: 0 };
  const siblings = categories
    .filter((c) => c.parentId === target.parentId)
    .sort((a, b) => a.sortIndex - b.sortIndex);
  return { parentId: target.parentId, index: Math.max(0, siblings.findIndex((c) => c.id === targetId)) };
}

function insertIndexAfterCategory(categories: Category[], targetId: string) {
  const x = insertIndexBeforeCategory(categories, targetId);
  return { ...x, index: x.index + 1 };
}

function insertIndexBeforeMod(mods: ModEntry[], targetId: string) {
  const target = mods.find((m) => m.id === targetId);
  if (!target) return { categoryId: null, index: 0 };
  const siblings = mods
    .filter((m) => m.categoryId === target.categoryId)
    .sort((a, b) => a.sortIndex - b.sortIndex);
  return { categoryId: target.categoryId, index: Math.max(0, siblings.findIndex((m) => m.id === targetId)) };
}

function insertIndexAfterMod(mods: ModEntry[], targetId: string) {
  const x = insertIndexBeforeMod(mods, targetId);
  return { ...x, index: x.index + 1 };
}

export default function App() {
  const [snapshot, setSnapshot] = useState<WorkspaceSnapshot>(emptySnapshot);
  const [error, setError] = useState<string | null>(null);
  const sensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: 5 } }));

  useEffect(() => {
    backend.getSnapshot().then(setSnapshot).catch((e) => setError(String(e)));
    let unlisten: (() => void) | undefined;
    backend.onSnapshotChanged(setSnapshot).then((fn) => { unlisten = fn; });
    return () => unlisten?.();
  }, []);

  const rootCategories = useMemo(
    () => snapshot.categories.filter((c) => c.parentId === null).sort((a, b) => a.sortIndex - b.sortIndex || a.name.localeCompare(b.name)),
    [snapshot.categories]
  );
  const rootMods = useMemo(
    () => snapshot.mods.filter((m) => m.categoryId === null).sort((a, b) => a.sortIndex - b.sortIndex || a.name.localeCompare(b.name)),
    [snapshot.mods]
  );

  const run = async (op: () => Promise<WorkspaceSnapshot>) => {
    try {
      setError(null);
      setSnapshot(await op());
    } catch (e) {
      setError(String(e));
    }
  };

  const chooseFolder = async () => {
    const selected = await open({ directory: true, multiple: false, title: "Выбери папку mods" });
    if (typeof selected === "string") await run(() => backend.setModsDirectory(selected));
  };

  const createCategory = async (parentId: string | null) => {
    const name = window.prompt("Название категории:")?.trim();
    if (name) await run(() => backend.createCategory(name, parentId));
  };

  const handleDragEnd = async ({ active, over }: DragEndEvent) => {
    if (!over) return;
    const item = active.data.current as DragItem | undefined;
    const target = over.data.current as DropTarget | undefined;
    if (!item || !target) return;

    if (item.kind === "category") {
      if (target.kind === "root") {
        await run(() => backend.moveCategory(item.id, null, rootCategories.length));
      } else if (target.kind === "category-inside") {
        const childCount = snapshot.categories.filter((c) => c.parentId === target.categoryId).length;
        await run(() => backend.moveCategory(item.id, target.categoryId, childCount));
      } else if (target.kind === "category-before") {
        const x = insertIndexBeforeCategory(snapshot.categories, target.categoryId);
        await run(() => backend.moveCategory(item.id, x.parentId, x.index));
      } else if (target.kind === "category-after") {
        const x = insertIndexAfterCategory(snapshot.categories, target.categoryId);
        await run(() => backend.moveCategory(item.id, x.parentId, x.index));
      }
      return;
    }

    if (item.kind === "mod") {
      if (target.kind === "root") {
        await run(() => backend.moveMod(item.id, null, rootMods.length));
      } else if (target.kind === "category-inside") {
        const count = snapshot.mods.filter((m) => m.categoryId === target.categoryId).length;
        await run(() => backend.moveMod(item.id, target.categoryId, count));
      } else if (target.kind === "mod-before") {
        const x = insertIndexBeforeMod(snapshot.mods, target.modId);
        await run(() => backend.moveMod(item.id, x.categoryId, x.index));
      } else if (target.kind === "mod-after") {
        const x = insertIndexAfterMod(snapshot.mods, target.modId);
        await run(() => backend.moveMod(item.id, x.categoryId, x.index));
      }
    }
  };

  return (
    <DndContext sensors={sensors} onDragEnd={handleDragEnd}>
      <div className="app-shell">
        <header className="topbar">
          <div>
            <h1>Mod Organizer</h1>
            <div className="path">{snapshot.modsDir ?? "Папка mods не выбрана"}</div>
          </div>
          <div className="toolbar">
            <button onClick={() => void createCategory(null)}>
              <span className="nf-icon">{nerdGlyph(nerdIconHex.plus)}</span>
              Категория
            </button>
            <button onClick={() => void chooseFolder()}>
              <span className="nf-icon">{nerdGlyph(nerdIconHex.modsFolder)}</span>
              Выбрать mods
            </button>
            <button onClick={() => void run(backend.rescan)}>
              <span className="nf-icon">{nerdGlyph(nerdIconHex.refresh)}</span>
              Обновить
            </button>
          </div>
        </header>

        {error && <div className="error">{error}</div>}

        <main className="tree-panel">
          <DropZone id="root-drop" target={{ kind: "root" }} className="root-drop" />
          {rootCategories.map((category) => (
            <CategoryNode
              key={category.id}
              category={category}
              categories={snapshot.categories}
              mods={snapshot.mods}
              depth={0}
              inheritedColor={null}
              onCreateCategory={createCategory}
              onDeleteCategory={(id) => void run(() => backend.deleteCategory(id))}
              onSetCategoryColor={(id, color) => void run(() => backend.setCategoryColor(id, color))}
              onSetCategoryIcon={(id, iconHex) => void run(() => backend.setCategoryIcon(id, iconHex))}
              onToggleMod={(id, enabled) => void run(() => backend.setModEnabled(id, enabled))}
            />
          ))}
          {rootMods.map((mod) => (
            <ModRow
              key={mod.id}
              mod={mod}
              categoryColor={null}
              onToggle={(id, enabled) => void run(() => backend.setModEnabled(id, enabled))}
            />
          ))}
          {!snapshot.modsDir && (
            <div className="empty-state">
              <strong>Выбери папку Minecraft `mods`.</strong>
              <span>После этого новые JAR будут появляться здесь автоматически.</span>
            </div>
          )}
        </main>
      </div>
    </DndContext>
  );
}
