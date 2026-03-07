import { useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { useMeetingStore } from "@/store/meetingStore";
import type { StreamingSegment } from "@/types";

export function RealtimeTranscript() {
  const { realtimeSegments, appendRealtimeSegments } = useMeetingStore();
  const bottomRef = useRef<HTMLDivElement>(null);
  const bufferRef = useRef<StreamingSegment[]>([]);
  const rafRef = useRef<number>(0);

  useEffect(() => {
    const unlisten = listen<StreamingSegment>("funasr-segment", (event) => {
      bufferRef.current.push(event.payload);
      cancelAnimationFrame(rafRef.current);
      rafRef.current = requestAnimationFrame(() => {
        if (bufferRef.current.length > 0) {
          appendRealtimeSegments(bufferRef.current);
          bufferRef.current = [];
        }
      });
    });
    return () => {
      cancelAnimationFrame(rafRef.current);
      void unlisten.then((fn) => fn());
    };
  }, [appendRealtimeSegments]);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "instant" });
  }, [realtimeSegments]);

  if (realtimeSegments.length === 0) {
    return (
      <div className="w-full max-w-lg rounded-lg border border-border bg-muted/30 px-4 py-3 text-xs text-muted-foreground">
        等待实时转写…
      </div>
    );
  }

  return (
    <div className="w-full max-w-lg max-h-32 overflow-y-auto rounded-lg border border-border bg-muted/30 px-4 py-3 space-y-1">
      {realtimeSegments.map((seg, i) => (
        <p
          key={i}
          className={`text-xs leading-relaxed ${seg.is_final ? "text-foreground" : "text-muted-foreground italic"}`}
        >
          {seg.text}
        </p>
      ))}
      <div ref={bottomRef} />
    </div>
  );
}
