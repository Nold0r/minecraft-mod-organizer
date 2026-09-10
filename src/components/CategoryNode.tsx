import { useDraggable, useDroppable } from "@dnd-kit/core";
import { type CSSProperties, useState } from "react";
import { nerdGlyph, nerdIconHex, normalizeColorHex, normalizeIconHex } from "../icons";
import type { Category, ModEntry } from "../types";
import { CategoryContextMenu } from "./CategoryContextMenu";
import { DropZone } from "./DropZone";
import { ModRow } from "./ModRow";

export function CategoryNode({
  category,
  categories,
  mods,
  depth,
  inheritedColor,
  onCreateCategory,
  onDeleteCategory,
  onSetCategoryColor,
  onSetCategoryIcon,
  onToggleMod,
}: {
  category: Category;
  categories: Category[];
  mods: ModEntry[];
  depth: number;
  inheritedColor: string | null;
  onCreateCategory: (parentId: string | null) => void;
  onDeleteCategory: (id: string) => void;
  onSetCategoryColor: (id: string, color: string | null) => void;
  onSetCategoryIcon: (id: string, iconHex: string | null) => void;
  onToggleMod: (id: string, enabled: boolean) => void;
}) {
  const [open, setOpen] = useState(true);
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number } | null>(null);
  const { attributes, listeners, setNodeRef: setDragRef, transform, isDragging } = useDraggable({
    id: `category:${category.id}`,
    data: { kind: "category", id: category.id },
  });
  const { setNodeRef: setDropRef, isOver } = useDroppable({
    id: `category-inside:${category.id}`,
    data: { kind: "category-inside", categoryId: category.id },
  });

  const children = categories
    .filter((c) => c.parentId === category.id)
    .sort((a, b) => a.sortIndex - b.sortIndex || a.name.localeCompare(b.name));
  const ownMods = mods
    .filter((m) => m.categoryId === category.id)
    .sort((a, b) => a.sortIndex - b.sortIndex || a.name.localeCompare(b.name));

  const effectiveColor = category.color ?? inheritedColor;
  const transformStyle = transform
    ? { transform: `translate3d(${transform.x}px, ${transform.y}px, 0)` }
    : {};
  const rowStyle = {
    ...transformStyle,
    marginLeft: depth * 14,
    ...(effectiveColor ? { "--category-color": effectiveColor } : {}),
  } as CSSProperties;

  const askForColor = () => {
    setContextMenu(null);
    const value = window.prompt(
      "HEX-цвет категории, например #7A4DFF. Оставь пустым, чтобы наследовать цвет родителя:",
      category.color ?? "",
    );
    if (value === null) return;
    if (!value.trim()) {
      onSetCategoryColor(category.id, null);
      return;
    }
    const normalized = normalizeColorHex(value);
    if (!normalized) {
      window.alert("Цвет должен быть в формате #RRGGBB, например #7A4DFF.");
      return;
    }
    onSetCategoryColor(category.id, normalized);
  };

  const askForIcon = () => {
    setContextMenu(null);
    const value = window.prompt(
      "HEX-код глифа JetBrains Nerd Font, например F07B. Оставь пустым для стандартной папки:",
      category.iconHex ?? "",
    );
    if (value === null) return;
    if (!value.trim()) {
      onSetCategoryIcon(category.id, null);
      return;
    }
    const normalized = normalizeIconHex(value);
    if (!normalized) {
      window.alert("Укажи HEX-код Unicode-глифа, например F07B, 0xF07B или U+F07B.");
      return;
    }
    onSetCategoryIcon(category.id, normalized);
  };

  const requestDelete = () => {
    const confirmed = window.confirm(
      `Удалить категорию «${category.name}» и все вложенные категории?\n\nМоды и JAR-файлы удалены не будут — они вернутся в корень списка.`,
    );
    if (confirmed) onDeleteCategory(category.id);
  };

  return (
    <div className="category-block">
      <DropZone
        id={`category-before:${category.id}`}
        target={{ kind: "category-before", categoryId: category.id }}
      />
      <div
        ref={(node) => {
          setDragRef(node);
          setDropRef(node);
        }}
        style={rowStyle}
        className={`category-row ${effectiveColor ? "has-category-color" : ""} ${isDragging ? "dragging" : ""} ${isOver ? "inside-over" : ""}`}
        onContextMenu={(event) => {
          event.preventDefault();
          event.stopPropagation();
          setContextMenu({
            x: Math.min(event.clientX, window.innerWidth - 260),
            y: Math.min(event.clientY, window.innerHeight - 112),
          });
        }}
      >
        <button className="icon-button collapse" onClick={() => setOpen((v) => !v)} title={open ? "Свернуть" : "Развернуть"}>
          <span className="nf-icon">{nerdGlyph(open ? nerdIconHex.chevronDown : nerdIconHex.chevronRight)}</span>
        </button>
        <button className="icon-button drag-handle" title="Перетащить категорию" {...listeners} {...attributes}>
          <span className="nf-icon">{nerdGlyph(nerdIconHex.grip)}</span>
        </button>
        <span className="nf-icon folder" title={`Иконка U+${(category.iconHex ?? nerdIconHex.folder).toUpperCase()}`}>
          {nerdGlyph(category.iconHex, nerdIconHex.folder)}
        </span>
        <span className="category-name">{category.name}</span>
        <span className="category-count">{children.length + ownMods.length}</span>
        <button className="icon-button inline-add" onClick={() => onCreateCategory(category.id)} title="Новая вложенная категория">
          <span className="nf-icon">{nerdGlyph(nerdIconHex.plus)}</span>
        </button>
        <button className="icon-button delete-category" onClick={requestDelete} title="Удалить категорию">
          <span className="nf-icon">{nerdGlyph(nerdIconHex.trash)}</span>
        </button>
      </div>

      {contextMenu && (
        <CategoryContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          onSetColor={askForColor}
          onSetIcon={askForIcon}
          onClose={() => setContextMenu(null)}
        />
      )}

      {open && (
        <div className="category-children">
          {children.map((child) => (
            <CategoryNode
              key={child.id}
              category={child}
              categories={categories}
              mods={mods}
              depth={depth + 1}
              inheritedColor={effectiveColor}
              onCreateCategory={onCreateCategory}
              onDeleteCategory={onDeleteCategory}
              onSetCategoryColor={onSetCategoryColor}
              onSetCategoryIcon={onSetCategoryIcon}
              onToggleMod={onToggleMod}
            />
          ))}
          <div style={{ marginLeft: (depth + 1) * 14 }}>
            {ownMods.map((mod) => (
              <ModRow key={mod.id} mod={mod} categoryColor={effectiveColor} onToggle={onToggleMod} />
            ))}
          </div>
        </div>
      )}

      <DropZone
        id={`category-after:${category.id}`}
        target={{ kind: "category-after", categoryId: category.id }}
      />
    </div>
  );
}
