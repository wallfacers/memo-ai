import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useMeetingStore } from "../store/meetingStore";

export function useRecording(meetingId: number | null) {
  const [isRecording, setIsRecording] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const { setCurrentMeetingStatus } = useMeetingStore();

  const startRecording = useCallback(async () => {
    if (!meetingId) return;
    try {
      setError(null);
      await invoke("start_recording", { meetingId });
      setIsRecording(true);
      setCurrentMeetingStatus("recording");
    } catch (e) {
      setError(String(e));
    }
  }, [meetingId, setCurrentMeetingStatus]);

  const stopRecording = useCallback(async () => {
    if (!meetingId) return;
    try {
      const audioPath = await invoke<string>("stop_recording", { meetingId });
      setIsRecording(false);
      setCurrentMeetingStatus("processing");
      return audioPath;
    } catch (e) {
      setError(String(e));
      setIsRecording(false);
    }
  }, [meetingId, setCurrentMeetingStatus]);

  return { isRecording, error, startRecording, stopRecording };
}
