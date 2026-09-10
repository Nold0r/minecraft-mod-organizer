import { useDroppable } from "@dnd-kit/core";
import type { DropTarget } from "../types";

export function DropZone({ id, target, className = "drop-zone" }: {
  id: string;
  target: DropTarget;
  className?: string;
}) {
  const { setNodeRef, isOver } = useDroppable({ id, data: target });
  return <div ref={setNodeRef} className={`${className} ${isOver ? "is-over" : ""}`} />;
}
