import { useEffect, useRef } from "react";
import type { Transcript } from "@/types";
import { formatTimestamp } from "@/utils/format";

interface TranscriptViewProps {
  transcripts: Transcript[];
}

export function TranscriptView({ transcripts }: TranscriptViewProps) {
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [transcripts]);

  if (transcripts.length === 0) {
    return (
      <div className="flex items-center justify-center py-12 text-sm text-muted-foreground">
        暂无转写内容
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-3 pb-4">
      {transcripts.map((t) => (
        <div key={t.id} className="flex gap-3">
          <span className="mt-0.5 shrink-0 text-[11px] tabular-nums text-muted-foreground w-10">
            {formatTimestamp(t.timestamp)}
          </span>
          <div className="text-sm leading-relaxed">
            {t.speaker && (
              <span className="font-semibold text-primary mr-1">{t.speaker}：</span>
            )}
            <span className="text-foreground">{t.text}</span>
          </div>
        </div>
      ))}
      <div ref={bottomRef} />
    </div>
  );
}
