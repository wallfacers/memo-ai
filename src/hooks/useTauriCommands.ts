import { invoke } from "@tauri-apps/api/core";
import type { Meeting, Transcript, ActionItem, AppSettings, PipelineResult } from "../types";

// Meeting commands
export function useListMeetings() {
  return () => invoke<Meeting[]>("list_meetings");
}

export function useGetMeeting() {
  return (id: number) => invoke<Meeting>("get_meeting", { id });
}

export function useCreateMeeting() {
  return (title: string, autoTitled: boolean = false) =>
    invoke<Meeting>("create_meeting", { title, autoTitled });
}

export function useDeleteMeeting() {
  return (id: number) => invoke<void>("delete_meeting", { id });
}

// Recording commands
export function useStartRecording() {
  return (meetingId: number) => invoke<void>("start_recording", { meetingId });
}

export function useStopRecording() {
  return (meetingId: number) => invoke<string>("stop_recording", { meetingId });
}

// Transcript commands
export function useGetTranscripts() {
  return (meetingId: number) => invoke<Transcript[]>("get_transcripts", { meetingId });
}

// ASR command
export function useTranscribeAudio() {
  return (audioPath: string, meetingId: number) =>
    invoke<string>("transcribe_audio", { audioPath, meetingId });
}

// Pipeline command
export function useRunPipeline() {
  return (meetingId: number) => invoke<PipelineResult>("run_pipeline", { meetingId });
}

// Action items commands
export function useGetActionItems() {
  return (meetingId: number) => invoke<ActionItem[]>("get_action_items", { meetingId });
}

export function useUpdateActionItemStatus() {
  return (id: number, status: "pending" | "done") =>
    invoke<void>("update_action_item_status", { id, status });
}

// Settings commands
export function useGetSettings() {
  return () => invoke<AppSettings>("get_settings");
}

export function useSaveSettings() {
  return (settings: AppSettings) => invoke<void>("save_settings", { settings });
}
