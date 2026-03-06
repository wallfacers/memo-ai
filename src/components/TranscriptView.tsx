import React, { useEffect, useRef } from "react";
import type { Transcript } from "../types";
import { formatTimestamp } from "../utils/format";

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
      <div style={{ color: "#6b7280", textAlign: "center", padding: "2rem" }}>
        暂无转写内容
      </div>
    );
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12, overflowY: "auto", maxHeight: 400 }}>
      {transcripts.map((t) => (
        <div key={t.id} style={{ display: "flex", gap: 12 }}>
          <span style={{ color: "#9ca3af", fontSize: 12, minWidth: 40, paddingTop: 2 }}>
            {formatTimestamp(t.timestamp)}
          </span>
          <div>
            {t.speaker && (
              <span style={{ fontWeight: 600, color: "#3b82f6", marginRight: 8 }}>
                {t.speaker}：
              </span>
            )}
            <span style={{ color: "#1f2937" }}>{t.text}</span>
          </div>
        </div>
      ))}
      <div ref={bottomRef} />
    </div>
  );
}
