import { create } from "zustand";
import type { Meeting, Transcript, ActionItem, MeetingStatus, RecordingPhase, StreamingSegment, PipelineStageDoneEvent } from "../types";

interface MeetingStore {
  meetings: Meeting[];
  currentMeeting: Meeting | null;
  transcripts: Transcript[];
  actionItems: ActionItem[];
  isLoading: boolean;
  error: string | null;
  recordingPhase: RecordingPhase;
  realtimeSegments: StreamingSegment[];
  pipelineStages: PipelineStageDoneEvent[];

  setMeetings: (meetings: Meeting[]) => void;
  setCurrentMeeting: (meeting: Meeting | null) => void;
  updateCurrentMeeting: (meeting: Meeting) => void;
  setCurrentMeetingStatus: (status: MeetingStatus) => void;
  updateMeetingTitle: (id: number, title: string) => void;
  setTranscripts: (transcripts: Transcript[]) => void;
  appendTranscript: (transcript: Transcript) => void;
  setActionItems: (items: ActionItem[]) => void;
  setLoading: (loading: boolean) => void;
  setError: (error: string | null) => void;
  setRecordingPhase: (phase: RecordingPhase) => void;
  appendRealtimeSegment: (segment: StreamingSegment) => void;
  appendRealtimeSegments: (segments: StreamingSegment[]) => void;
  clearRealtimeSegments: () => void;
  appendPipelineStage: (stage: PipelineStageDoneEvent) => void;
  clearPipelineStages: () => void;
  pipelineFailedStage: { stage: number; error: string } | null;
  setPipelineFailedStage: (info: { stage: number; error: string } | null) => void;
}

export const useMeetingStore = create<MeetingStore>((set) => ({
  meetings: [],
  currentMeeting: null,
  transcripts: [],
  actionItems: [],
  isLoading: false,
  error: null,
  recordingPhase: "idle",
  realtimeSegments: [],
  pipelineStages: [],
  pipelineFailedStage: null,

  setMeetings: (meetings) => set({ meetings }),
  setCurrentMeeting: (meeting) => set({ currentMeeting: meeting, transcripts: [], actionItems: [] }),
  updateCurrentMeeting: (meeting) => set({ currentMeeting: meeting }),
  setCurrentMeetingStatus: (status) =>
    set((state) => ({
      currentMeeting: state.currentMeeting ? { ...state.currentMeeting, status } : null,
      meetings: state.meetings.map((m) =>
        state.currentMeeting && m.id === state.currentMeeting.id ? { ...m, status } : m
      ),
    })),
  updateMeetingTitle: (id, title) =>
    set((state) => ({
      meetings: state.meetings.map((m) => (m.id === id ? { ...m, title } : m)),
      currentMeeting:
        state.currentMeeting?.id === id
          ? { ...state.currentMeeting, title }
          : state.currentMeeting,
    })),
  setTranscripts: (transcripts) => set({ transcripts }),
  appendTranscript: (transcript) =>
    set((state) => ({ transcripts: [...state.transcripts, transcript] })),
  setActionItems: (actionItems) => set({ actionItems }),
  setLoading: (isLoading) => set({ isLoading }),
  setError: (error) => set({ error }),
  setRecordingPhase: (phase) => set({ recordingPhase: phase }),
  appendRealtimeSegment: (segment) =>
    set((state) => ({ realtimeSegments: [...state.realtimeSegments, segment] })),
  appendRealtimeSegments: (segments) =>
    set((state) => ({ realtimeSegments: [...state.realtimeSegments, ...segments] })),
  clearRealtimeSegments: () => set({ realtimeSegments: [] }),
  appendPipelineStage: (stage) =>
    set((state) => ({ pipelineStages: [...state.pipelineStages, stage] })),
  clearPipelineStages: () => set({ pipelineStages: [] }),
  setPipelineFailedStage: (info) => set({ pipelineFailedStage: info }),
}));
