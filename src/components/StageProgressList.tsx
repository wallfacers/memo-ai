import { CheckCircle2, Loader2 } from "lucide-react";

export interface StageItem {
  key: string | number;
  label: string;
  status: "done" | "active" | "pending";
}

interface StageProgressListProps {
  stages: StageItem[];
  title?: string;
}

export function StageProgressList({ stages, title }: StageProgressListProps) {
  return (
    <div className="w-full rounded-lg border border-border bg-muted/30 px-4 py-3 space-y-2">
      {title && <p className="text-xs font-medium text-foreground">{title}</p>}
      <div className="space-y-1">
        {stages.map((stage) => (
          <div key={stage.key} className="flex items-center gap-2 text-xs">
            {stage.status === "done" ? (
              <CheckCircle2 className="h-3.5 w-3.5 shrink-0 text-green-600" />
            ) : stage.status === "active" ? (
              <Loader2 className="h-3.5 w-3.5 shrink-0 animate-spin text-muted-foreground" />
            ) : (
              <span className="h-3.5 w-3.5 shrink-0 rounded-full border border-muted-foreground/30" />
            )}
            <span className={stage.status === "done" ? "text-foreground" : "text-muted-foreground"}>
              {stage.label}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}
