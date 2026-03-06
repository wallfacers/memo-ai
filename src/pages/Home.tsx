import React, { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import { MeetingCard } from "../components/MeetingCard";
import { useMeetingStore } from "../store/meetingStore";
import type { Meeting } from "../types";

export function Home() {
  const navigate = useNavigate();
  const { meetings, setMeetings, isLoading, setLoading, setError } = useMeetingStore();
  const [newTitle, setNewTitle] = useState("");

  useEffect(() => {
    loadMeetings();
  }, []);

  async function loadMeetings() {
    setLoading(true);
    try {
      const data = await invoke<Meeting[]>("list_meetings");
      setMeetings(data);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  async function createMeeting() {
    const title = newTitle.trim() || `会议 ${new Date().toLocaleString("zh-CN")}`;
    try {
      const meeting = await invoke<Meeting>("create_meeting", { title });
      navigate(`/meeting/${meeting.id}`);
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <div style={{ maxWidth: 800, margin: "0 auto", padding: "32px 20px" }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 24 }}>
        <h1 style={{ margin: 0, fontSize: 28, fontWeight: 700, color: "#1f2937" }}>
          Memo AI
          <span style={{ fontSize: 14, fontWeight: 400, color: "#6b7280", marginLeft: 8 }}>
            AI 会议助手
          </span>
        </h1>
        <button
          onClick={() => navigate("/settings")}
          style={{
            padding: "8px 16px",
            background: "transparent",
            border: "1px solid #e5e7eb",
            borderRadius: 8,
            cursor: "pointer",
            color: "#6b7280",
            fontSize: 14,
          }}
        >
          ⚙ 设置
        </button>
      </div>

      <div style={{ display: "flex", gap: 12, marginBottom: 32 }}>
        <input
          type="text"
          value={newTitle}
          onChange={(e) => setNewTitle(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && createMeeting()}
          placeholder="会议标题（回车创建）"
          style={{
            flex: 1,
            padding: "10px 14px",
            borderRadius: 8,
            border: "1px solid #d1d5db",
            fontSize: 14,
            outline: "none",
          }}
        />
        <button
          onClick={createMeeting}
          style={{
            padding: "10px 20px",
            background: "#3b82f6",
            color: "#fff",
            border: "none",
            borderRadius: 8,
            cursor: "pointer",
            fontSize: 14,
            fontWeight: 500,
          }}
        >
          + 新会议
        </button>
      </div>

      {isLoading ? (
        <div style={{ textAlign: "center", color: "#6b7280", padding: "3rem" }}>加载中...</div>
      ) : meetings.length === 0 ? (
        <div style={{ textAlign: "center", color: "#6b7280", padding: "4rem" }}>
          <div style={{ fontSize: 48, marginBottom: 12 }}>🎙</div>
          <div style={{ fontSize: 16 }}>暂无会议记录</div>
          <div style={{ fontSize: 14, marginTop: 8 }}>点击「+ 新会议」开始录制</div>
        </div>
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
          {meetings.map((m) => (
            <MeetingCard key={m.id} meeting={m} />
          ))}
        </div>
      )}
    </div>
  );
}
