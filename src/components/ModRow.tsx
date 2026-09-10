import { useDraggable } from "@dnd-kit/core";
import type { CSSProperties } from "react";
import { nerdGlyph, nerdIconHex } from "../icons";
import type { ModEntry } from "../types";
import { DropZone } from "./DropZone";

export function ModRow({ mod, categoryColor, onToggle }: {
  mod: ModEntry;
  categoryColor: string | null;
  onToggle: (id: string, enabled: boolean) => void;
}) {
  const { attributes, listeners, setNodeRef, transform, isDragging } = useDraggable({
    id: `mod:${mod.id}`,
    data: { kind: "mod", id: mod.id },
  });

  const style = {
    ...(transform
      ? { transform: `translate3d(${transform.x}px, ${transform.y}px, 0)` }
      : {}),
    ...(categoryColor ? { "--category-color": categoryColor } : {}),
  } as CSSProperties;

  const packagedMods = mod.packagedMods ?? [];
  const hasMultiplePackages = packagedMods.length > 1;

  return (
    <>
      <DropZone id={`mod-before:${mod.id}`} target={{ kind: "mod-before", modId: mod.id }} />
      <div
        ref={setNodeRef}
        style={style}
        className={`mod-row ${categoryColor ? "has-category-color" : ""} ${isDragging ? "dragging" : ""} ${!mod.enabled ? "disabled" : ""}`}
      >
        <button className="icon-button drag-handle" title="Перетащить" {...listeners} {...attributes}>
          <span className="nf-icon">{nerdGlyph(nerdIconHex.grip)}</span>
        </button>
        <div className="mod-icon">
          {mod.iconDataUrl ? <img src={mod.iconDataUrl} alt="" /> : <span className="nf-icon">{nerdGlyph(nerdIconHex.image)}</span>}
        </div>
        <div className="mod-main">
          <div className="mod-title">{mod.name}</div>
          <div className="mod-meta">
            <span>{mod.version || "без версии"}</span>
            <span className={`loader loader-${mod.loader.toLowerCase()}`}>{mod.loader}</span>
            <span className="mod-id">{mod.modId}</span>
          </div>
        </div>

        {hasMultiplePackages && (
          <div className="packaged-mods-control" tabIndex={0} aria-label={`Пакеты в ${mod.fileName}`}>
            <span className="nf-icon packaged-mods-icon">{nerdGlyph(nerdIconHex.packagedMods)}</span>
            <div className="packaged-mods-tooltip" role="tooltip">
              <div className="packaged-mods-title">Пакеты в этом jar</div>
              <div className="packaged-mods-file">{mod.fileName}</div>
              <div className="packaged-mods-list">
                {packagedMods.map((packed, index) => (
                  <div className="packaged-mod-item" key={`${packed.loader}:${packed.modId}:${packed.version}:${index}`}>
                    <div className="packaged-mod-name">{packed.name}</div>
                    <div className="packaged-mod-meta">
                      <span>{packed.modId}</span>
                      <span>{packed.version || "без версии"}</span>
                      <span>{packed.loader}</span>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </div>
        )}

        <label className="switch" title={mod.enabled ? "Выключить мод" : "Включить мод"}>
          <input
            type="checkbox"
            checked={mod.enabled}
            onChange={(e) => onToggle(mod.id, e.target.checked)}
          />
          <span className="slider" />
        </label>
      </div>
      <DropZone id={`mod-after:${mod.id}`} target={{ kind: "mod-after", modId: mod.id }} />
    </>
  );
}
