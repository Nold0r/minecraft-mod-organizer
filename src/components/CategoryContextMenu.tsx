import { useEffect, useRef } from "react";
import { nerdGlyph, nerdIconHex } from "../icons";

export function CategoryContextMenu({
  x,
  y,
  onSetColor,
  onSetIcon,
  onClose,
}: {
  x: number;
  y: number;
  onSetColor: () => void;
  onSetIcon: () => void;
  onClose: () => void;
}) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const close = (event: PointerEvent) => {
      if (!ref.current?.contains(event.target as Node)) onClose();
    };
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("pointerdown", close);
    window.addEventListener("keydown", closeOnEscape);
    return () => {
      window.removeEventListener("pointerdown", close);
      window.removeEventListener("keydown", closeOnEscape);
    };
  }, [onClose]);

  return (
    <div
      ref={ref}
      className="context-menu"
      style={{ left: x, top: y }}
      role="menu"
      onContextMenu={(event) => event.preventDefault()}
    >
      <button type="button" role="menuitem" onClick={onSetColor}>
        <span className="nf-icon">{nerdGlyph(nerdIconHex.palette)}</span>
        Задать цвет категории
      </button>
      <button type="button" role="menuitem" onClick={onSetIcon}>
        <span className="nf-icon">{nerdGlyph(nerdIconHex.image)}</span>
        Выбрать иконку
      </button>
    </div>
  );
}
