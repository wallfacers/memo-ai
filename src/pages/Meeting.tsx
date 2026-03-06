import React, { useEffect } from "react";
import { useParams, useNavigate } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import { RecordButton } from "../components/RecordButton";
import { TranscriptView } from "../components/TranscriptView";
import { ActionItemList } from "../components/ActionItemList";
import { useMeetingStore } from "../store/meetingStore";
import { useRecording } from "../hooks/useRecording";
import type { Meeting as MeetingType, Transcript, ActionItem } from "../types";

export function Meeting() {
  const { id } = useParams<{ id: string }>();
  const meetingId = id ? parseInt(id) : null;
  const navigate = useNavigate();

  const {
    currentMeeting,
    setCurrentMeeting,
    transcripts,
    setTranscripts,
    actionItems,
    setActionItems,
    setCurrentMeetingStatus,
  } = useMeetingStore();

  const { isRecording, error, startRecording, stopRecording } = useRecording(meetingId);

  useEffect(() => {
    if (!meetingId) return;
    loadMeeting();
    loadTranscripts();
    loadActionItems();
  }, [meetingId]);

  async function loadMeeting() {
    const meeting = await invoke<MeetingType>("get_meeting", { id: meetingId });
    setCurrentMeeting(meeting);
  }

  async function loadTranscripts() {
    const data = await invoke<Transcript[]>("get_transcripts", { meetingId });
    setTranscripts(data);
  }

  async function loadActionItems() {
    const data = await invoke<ActionItem[]>("get_action_items", { meetingId });
    setActionItems(data);
  }

  async function handleStopAndProcess() {
    const audioPath = await stopRecording();
    if (!audioPath || !meetingId) return;

    // Transcribe
    await invoke("transcribe_audio", { audioPath, meetingId });
    await loadTranscripts();

    // Run pipeline
    setCurrentMeetingStatus("processing");
    try {
      await invoke("run_pipeline", { meetingId });
      setCurrentMeetingStatus("completed");
      await loadMeeting();
      await loadActionItems();
    } catch (e) {
      setCurrentMeetingStatus("error");
    }
  }

  async function handleToggleActionItem(itemId: number, status: "pending" | "done") {
    await invoke("update_action_item_status", { id: itemId, status });
    await loadActionItems();
  }

  if (!currentMeeting) {
    return <div style={{ padding: 32, color: "#6b7280" }}>加载中...</div>;
  }

  return (
    <div style={{ maxWidth: 900, margin: "0 auto", padding: "24px 20px" }}>
      <button
        onClick={() => navigate("/")}
        style={{
          background: "none",
          border: "none",
          color: "#6b7280",
          cursor: "pointer",
          fontSize: 14,
          marginBottom: 16,
          padding: 0,
        }}
      >
        ← 返回列表
      </button>

      <h2 style={{ margin: "0 0 24px", fontSize: 22, fontWeight: 700, color: "#1f2937" }}>
        {currentMeeting.title}
      </h2>

      <div style={{ display: "flex", justifyContent: "center", marginBottom: 32 }}>
        <RecordButton
          isRecording={isRecording}
          disabled={currentMeeting.status === "processing"}
          onStart={startRecording}
          onStop={handleStopAndProcess}
        />
      </div>

      {error && (
        <div style={{ background: "#fef2f2", color: "#ef4444", padding: "12px 16px", borderRadius: 8, marginBottom: 16 }}>
          {error}
        </div>
      )}

      {currentMeeting.status === "processing" && (
        <div style={{ textAlign: "center", color: "#f59e0b", marginBottom: 16 }}>
          AI 正在处理会议内容...
        </div>
      )}

      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 24 }}>
        <section>
          <h3 style={{ fontSize: 16, fontWeight: 600, color: "#374151", marginBottom: 12 }}>实时转写</h3>
          <TranscriptView transcripts={transcripts} />
        </section>

        <section>
          <h3 style={{ fontSize: 16, fontWeight: 600, color: "#374151", marginBottom: 12 }}>行动项</h3>
          <ActionItemList items={actionItems} onToggle={handleToggleActionItem} />
        </section>
      </div>

      {currentMeeting.summary && (
        <section style={{ marginTop: 24 }}>
          <h3 style={{ fontSize: 16, fontWeight: 600, color: "#374151", marginBottom: 12 }}>会议总结</h3>
          <div style={{
            background: "#f0f9ff",
            border: "1px solid #bae6fd",
            borderRadius: 10,
            padding: "16px 20px",
            color: "#1e3a5f",
            lineHeight: 1.7,
            whiteSpace: "pre-wrap",
          }}>
            {currentMeeting.summary}
          </div>
        </section>
      )}

      {currentMeeting.report && (
        <section style={{ marginTop: 24 }}>
          <h3 style={{ fontSize: 16, fontWeight: 600, color: "#374151", marginBottom: 12 }}>会议报告</h3>
          <div style={{
            background: "#fafafa",
            border: "1px solid #e5e7eb",
            borderRadius: 10,
            padding: "16px 20px",
            color: "#374151",
            lineHeight: 1.7,
            whiteSpace: "pre-wrap",
            fontFamily: "monospace",
          }}>
            {currentMeeting.report}
          </div>
        </section>
      )}
    </div>
  );
}
