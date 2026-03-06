import React from "react";
import { Mic, Square } from "lucide-react";
import { cn } from "@/lib/utils";

interface RecordButtonProps {
  isRecording: boolean;
  disabled?: boolean;
  onStart: () => void;
  onStop: () => void;
}

export function RecordButton({ isRecording, disabled, onStart, onStop }: RecordButtonProps) {
  return (
    <button
      onClick={isRecording ? onStop : onStart}
      disabled={disabled}
      aria-label={isRecording ? "停止录音" : "开始录音"}
      className={cn(
        "flex h-20 w-20 items-center justify-center rounded-full transition-all duration-200",
        "focus:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2",
        isRecording
          ? "bg-destructive text-destructive-foreground recording-pulse"
          : "bg-primary text-primary-foreground shadow-lg hover:bg-primary/90 hover:shadow-xl active:scale-95",
        disabled && "cursor-not-allowed bg-muted text-muted-foreground shadow-none opacity-60"
      )}
    >
      {isRecording ? (
        <Square className="h-7 w-7 fill-current" />
      ) : (
        <Mic className="h-7 w-7" />
      )}
    </button>
  );
}
