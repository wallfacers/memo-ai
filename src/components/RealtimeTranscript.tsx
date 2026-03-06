import { useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { useMeetingStore } from "@/store/meetingStore";
import type { StreamingSegment } from "@/types";

export function RealtimeTranscript() {
  const { realtimeSegments, appendRealtimeSegment } = useMeetingStore();
  const bottomRef = useRef<HTMLDivElement>(null);

  // 监听 FunASR 实时字幕事件
  useEffect(() => {
    let unlistenPartial: (() => void) | undefined;
    let unlistenFinal: (() => void) | undefined;

    listen<StreamingSegment>("asr_partial", (event) => {
      appendRealtimeSegment(event.payload);
    }).then((fn) => {
      unlistenPartial = fn;
    });

    listen<StreamingSegment>("asr_final", (event) => {
      appendRealtimeSegment(event.payload);
    }).then((fn) => {
      unlistenFinal = fn;
    });

    return () => {
      unlistenPartial?.();
      unlistenFinal?.();
    };
  }, [appendRealtimeSegment]);

  // 自动滚动到底部
  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [realtimeSegments]);

  if (realtimeSegments.length === 0) {
    return (
      <div className="flex items-center justify-center h-20 text-sm text-muted-foreground">
        等待语音输入…
      </div>
    );
  }

  return (
    <div className="space-y-0.5 p-3 text-sm leading-relaxed overflow-y-auto max-h-48 rounded-md border bg-muted/30">
      {realtimeSegments.map((seg) => (
        <span
          key={seg.segment_id}
          className={
            seg.is_final
              ? "text-foreground"
              : "text-muted-foreground"
          }
        >
          {seg.text}
          {!seg.is_final && (
            <span className="inline-block w-px h-4 bg-primary ml-0.5 align-middle animate-pulse" />
          )}
          {" "}
        </span>
      ))}
      <div ref={bottomRef} />
    </div>
  );
}
