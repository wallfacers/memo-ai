import { useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Meeting, Transcript, ActionItem, AppSettings, PipelineResult, StreamingSegment } from "../types";

// Meeting commands
export function useListMeetings() {
  return useCallback(() => invoke<Meeting[]>("list_meetings"), []);
}

export function useGetMeeting() {
  return useCallback((id: number) => invoke<Meeting>("get_meeting", { id }), []);
}

export function useCreateMeeting() {
  return useCallback(
    (title: string, autoTitled: boolean = false) =>
      invoke<Meeting>("create_meeting", { title, autoTitled }),
    []
  );
}

export function useDeleteMeeting() {
  return useCallback((id: number) => invoke<void>("delete_meeting", { id }), []);
}

export function useRenameMeeting() {
  return useCallback((id: number, title: string) => invoke<void>("rename_meeting", { id, title }), []);
}

// Recording commands
export function useStartRecording() {
  return useCallback((meetingId: number) => invoke<void>("start_recording", { meetingId }), []);
}

export function useStopRecording() {
  return useCallback((meetingId: number) => invoke<string>("stop_recording", { meetingId }), []);
}

// Transcript commands
export function useGetTranscripts() {
  return useCallback(
    (meetingId: number) => invoke<Transcript[]>("get_transcripts", { meetingId }),
    []
  );
}

// ASR command
export function useTranscribeAudio() {
  return useCallback(
    (audioPath: string, meetingId: number) =>
      invoke<string>("transcribe_audio", { audioPath, meetingId }),
    []
  );
}

// Pipeline command
export function useRunPipeline() {
  return useCallback(
    (meetingId: number) => invoke<PipelineResult>("run_pipeline", { meetingId }),
    []
  );
}

// Action items commands
export function useGetActionItems() {
  return useCallback(
    (meetingId: number) => invoke<ActionItem[]>("get_action_items", { meetingId }),
    []
  );
}

export function useUpdateActionItemStatus() {
  return useCallback(
    (id: number, status: "pending" | "done") =>
      invoke<void>("update_action_item_status", { id, status }),
    []
  );
}

// Export / Search commands
export async function exportReport(meetingId: number, path: string): Promise<void> {
  await invoke<void>("export_report", { meetingId, path });
}

export async function searchMeetings(query: string): Promise<Meeting[]> {
  return await invoke<Meeting[]>("search_meetings", { query });
}

// Settings commands
export function useGetSettings() {
  return useCallback(() => invoke<AppSettings>("get_settings"), []);
}

export function useSaveSettings() {
  return useCallback(
    (settings: AppSettings) => invoke<void>("save_settings", { settings }),
    []
  );
}

export interface LlmTestResult {
  success: boolean;
  message: string;
  latency_ms: number;
}

export function useTestLlmConnection() {
  return useCallback(
    (settings: AppSettings) =>
      invoke<LlmTestResult>("test_llm_connection", { settings }),
    []
  );
}

export interface WhisperCheckResult {
  found: boolean;
  version: string | null;
  status: string;
}

export interface AsrTestResult {
  success: boolean;
  message: string;
}

export function useCheckWhisperCli() {
  return useCallback(
    (cliPath: string) =>
      invoke<WhisperCheckResult>("check_whisper_cli", { cliPath }),
    []
  );
}

export function useTestAsrConnection() {
  return useCallback(
    (settings: AppSettings) =>
      invoke<AsrTestResult>("test_asr_connection", { settings }),
    []
  );
}

// FunASR hooks
export interface FunAsrCheckResult {
  found: boolean;
  message: string;
}

export interface FunAsrStopResult {
  segments: StreamingSegment[];
}

export function useStartFunAsrSession() {
  return useCallback(
    (meetingId: number) =>
      invoke<void>("start_funasr_session", { meetingId }),
    []
  );
}

export function useStopFunAsrSession() {
  return useCallback(
    () => invoke<FunAsrStopResult>("stop_funasr_session"),
    []
  );
}

export function useCheckFunAsrServer() {
  return useCallback(
    (serverPath: string) =>
      invoke<FunAsrCheckResult>("check_funasr_server", { serverPath }),
    []
  );
}
