import { useState, useCallback } from "react";
import { useStartRecording, useStopRecording } from "./useTauriCommands";
import { useMeetingStore } from "../store/meetingStore";

export function useRecording(meetingId: number | null) {
  const [isRecording, setIsRecording] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const { setCurrentMeetingStatus } = useMeetingStore();
  const startRecordingCmd = useStartRecording();
  const stopRecordingCmd = useStopRecording();

  const startRecording = useCallback(async () => {
    if (!meetingId) return;
    try {
      setError(null);
      await startRecordingCmd(meetingId);
      setIsRecording(true);
      setCurrentMeetingStatus("recording");
    } catch (e) {
      setError(String(e));
    }
  }, [meetingId, startRecordingCmd, setCurrentMeetingStatus]);

  const stopRecording = useCallback(async () => {
    if (!meetingId) return;
    try {
      const audioPath = await stopRecordingCmd(meetingId);
      setIsRecording(false);
      setCurrentMeetingStatus("processing");
      return audioPath;
    } catch (e) {
      setError(String(e));
      setIsRecording(false);
    }
  }, [meetingId, stopRecordingCmd, setCurrentMeetingStatus]);

  return { isRecording, error, startRecording, stopRecording };
}
