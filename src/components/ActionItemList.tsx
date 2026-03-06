import React from "react";
import type { ActionItem } from "../types";

interface ActionItemListProps {
  items: ActionItem[];
  onToggle?: (id: number, status: "pending" | "done") => void;
}

export function ActionItemList({ items, onToggle }: ActionItemListProps) {
  if (items.length === 0) {
    return <div style={{ color: "#6b7280", textAlign: "center", padding: "1rem" }}>暂无行动项</div>;
  }

  return (
    <ul style={{ listStyle: "none", padding: 0, margin: 0, display: "flex", flexDirection: "column", gap: 8 }}>
      {items.map((item) => (
        <li
          key={item.id}
          style={{
            display: "flex",
            alignItems: "flex-start",
            gap: 12,
            padding: "10px 14px",
            background: item.status === "done" ? "#f0fdf4" : "#f9fafb",
            borderRadius: 8,
            border: `1px solid ${item.status === "done" ? "#86efac" : "#e5e7eb"}`,
          }}
        >
          <input
            type="checkbox"
            checked={item.status === "done"}
            onChange={() => onToggle?.(item.id, item.status === "done" ? "pending" : "done")}
            style={{ marginTop: 2, cursor: "pointer" }}
          />
          <div style={{ flex: 1 }}>
            <div style={{
              textDecoration: item.status === "done" ? "line-through" : "none",
              color: item.status === "done" ? "#6b7280" : "#1f2937",
              fontWeight: 500,
            }}>
              {item.task}
            </div>
            <div style={{ fontSize: 12, color: "#6b7280", marginTop: 4, display: "flex", gap: 12 }}>
              {item.owner && <span>负责人：{item.owner}</span>}
              {item.deadline && <span>截止：{item.deadline}</span>}
            </div>
          </div>
        </li>
      ))}
    </ul>
  );
}
