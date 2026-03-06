import React from "react";
import { useNavigate } from "react-router-dom";
import type { Meeting } from "../types";
import { formatDateTime, formatDuration, truncate } from "../utils/format";

interface MeetingCardProps {
  meeting: Meeting;
}

const statusLabel: Record<Meeting["status"], string> = {
  idle: "待录音",
  recording: "录音中",
  processing: "处理中",
  completed: "已完成",
  error: "出错",
};

const statusColor: Record<Meeting["status"], string> = {
  idle: "#6b7280",
  recording: "#ef4444",
  processing: "#f59e0b",
  completed: "#10b981",
  error: "#ef4444",
};

export function MeetingCard({ meeting }: MeetingCardProps) {
  const navigate = useNavigate();

  return (
    <div
      onClick={() => navigate(`/meeting/${meeting.id}`)}
      style={{
        padding: "16px 20px",
        background: "#fff",
        borderRadius: 12,
        border: "1px solid #e5e7eb",
        cursor: "pointer",
        transition: "box-shadow 0.15s",
        boxShadow: "0 1px 3px rgba(0,0,0,0.06)",
      }}
      onMouseEnter={(e) => (e.currentTarget.style.boxShadow = "0 4px 12px rgba(0,0,0,0.1)")}
      onMouseLeave={(e) => (e.currentTarget.style.boxShadow = "0 1px 3px rgba(0,0,0,0.06)")}
    >
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start" }}>
        <h3 style={{ margin: 0, fontSize: 16, fontWeight: 600, color: "#1f2937" }}>
          {meeting.title}
        </h3>
        <span style={{
          fontSize: 12,
          color: statusColor[meeting.status],
          background: `${statusColor[meeting.status]}18`,
          padding: "2px 10px",
          borderRadius: 12,
          fontWeight: 500,
        }}>
          {statusLabel[meeting.status]}
        </span>
      </div>
      <div style={{ fontSize: 13, color: "#6b7280", marginTop: 8, display: "flex", gap: 16 }}>
        <span>{formatDateTime(meeting.start_time)}</span>
        <span>{formatDuration(meeting.start_time, meeting.end_time)}</span>
      </div>
      {meeting.summary && (
        <p style={{ margin: "10px 0 0", fontSize: 14, color: "#4b5563", lineHeight: 1.5 }}>
          {truncate(meeting.summary, 100)}
        </p>
      )}
    </div>
  );
}
