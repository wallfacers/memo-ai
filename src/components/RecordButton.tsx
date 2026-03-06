import React from "react";

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
      style={{
        width: 72,
        height: 72,
        borderRadius: "50%",
        border: "none",
        cursor: disabled ? "not-allowed" : "pointer",
        background: isRecording ? "#ef4444" : "#3b82f6",
        color: "#fff",
        fontSize: 24,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        boxShadow: isRecording ? "0 0 0 8px rgba(239,68,68,0.3)" : "0 4px 12px rgba(0,0,0,0.2)",
        transition: "all 0.2s",
      }}
      aria-label={isRecording ? "停止录音" : "开始录音"}
    >
      {isRecording ? "⏹" : "🎙"}
    </button>
  );
}
