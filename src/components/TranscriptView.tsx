import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import type { Transcript } from "@/types";
import { formatTimestamp } from "@/utils/format";

interface TranscriptViewProps {
  transcripts: Transcript[];
}

export function TranscriptView({ transcripts }: TranscriptViewProps) {
  const { t } = useTranslation();
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [transcripts]);

  if (transcripts.length === 0) {
    return (
      <div className="flex items-center justify-center py-12 text-sm text-muted-foreground">
        {t("transcript.empty")}
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-3 pb-4">
      {transcripts.map((tr) => (
        <div key={tr.id} className="flex gap-3">
          <span className="mt-0.5 shrink-0 text-[11px] tabular-nums text-muted-foreground w-10">
            {formatTimestamp(tr.timestamp)}
          </span>
          <div className="text-sm leading-relaxed">
            {tr.speaker && (
              <span className="font-semibold text-primary mr-1">
                {tr.speaker}{t("transcript.speakerSuffix")}
              </span>
            )}
            <span className="text-foreground">{tr.text}</span>
          </div>
        </div>
      ))}
      <div ref={bottomRef} />
    </div>
  );
}
