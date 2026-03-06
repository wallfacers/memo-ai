import { Checkbox } from "@/components/ui/checkbox";
import { cn } from "@/lib/utils";
import type { ActionItem } from "@/types";

interface ActionItemListProps {
  items: ActionItem[];
  onToggle?: (id: number, status: "pending" | "done") => void;
}

export function ActionItemList({ items, onToggle }: ActionItemListProps) {
  if (items.length === 0) {
    return (
      <div className="flex items-center justify-center py-8 text-sm text-muted-foreground">
        暂无行动项
      </div>
    );
  }

  return (
    <ul className="flex flex-col gap-2">
      {items.map((item) => (
        <li
          key={item.id}
          className={cn(
            "flex items-start gap-3 rounded-lg border px-3 py-2.5 transition-colors",
            item.status === "done"
              ? "border-emerald-200 bg-emerald-50/50 dark:border-emerald-800 dark:bg-emerald-950/30"
              : "border-border bg-card"
          )}
        >
          <Checkbox
            id={`action-${item.id}`}
            checked={item.status === "done"}
            onCheckedChange={() =>
              onToggle?.(item.id, item.status === "done" ? "pending" : "done")
            }
            className="mt-0.5 shrink-0"
          />
          <div className="flex-1 min-w-0">
            <label
              htmlFor={`action-${item.id}`}
              className={cn(
                "block text-sm font-medium cursor-pointer",
                item.status === "done"
                  ? "line-through text-muted-foreground"
                  : "text-foreground"
              )}
            >
              {item.task}
            </label>
            {(item.owner || item.deadline) && (
              <div className="mt-1 flex gap-3 text-[11px] text-muted-foreground">
                {item.owner && <span>负责人：{item.owner}</span>}
                {item.deadline && <span>截止：{item.deadline}</span>}
              </div>
            )}
          </div>
        </li>
      ))}
    </ul>
  );
}
