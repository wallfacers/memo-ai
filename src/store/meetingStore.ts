import { create } from "zustand";
import type { Meeting, Transcript, ActionItem, MeetingStatus, RecordingPhase, StreamingSegment, PipelineStageDoneEvent } from "../types";

interface MeetingStore {
  meetings: Meeting[];
  currentMeeting: Meeting | null;
  transcripts: Transcript[];
  actionItems: ActionItem[];
  isLoading: boolean;
  error: string | null;

  setMeetings: (meetings: Meeting[]) => void;
  setCurrentMeeting: (meeting: Meeting | null) => void;
  setCurrentMeetingStatus: (status: MeetingStatus) => void;
  updateMeetingTitle: (id: number, title: string) => void;
  setTranscripts: (transcripts: Transcript[]) => void;
  appendTranscript: (transcript: Transcript) => void;
  setActionItems: (items: ActionItem[]) => void;
  setLoading: (loading: boolean) => void;
  setError: (error: string | null) => void;

  // 录音生命周期状态
  recordingPhase: RecordingPhase;
  recordingError: string | null;
  // 实时字幕（FunASR 流式结果）
  realtimeSegments: StreamingSegment[];
  // Pipeline 各阶段进度
  pipelineStages: PipelineStageDoneEvent[];

  // Actions
  setRecordingPhase: (phase: RecordingPhase, error?: string) => void;
  appendRealtimeSegment: (seg: StreamingSegment) => void;
  clearRealtimeSegments: () => void;
  appendPipelineStage: (stage: PipelineStageDoneEvent) => void;
  clearPipelineStages: () => void;
}

export const useMeetingStore = create<MeetingStore>((set) => ({
  meetings: [],
  currentMeeting: null,
  transcripts: [],
  actionItems: [],
  isLoading: false,
  error: null,

  setMeetings: (meetings) => set({ meetings }),
  setCurrentMeeting: (meeting) => set({ currentMeeting: meeting, transcripts: [], actionItems: [] }),
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

  recordingPhase: "idle",
  recordingError: null,
  realtimeSegments: [],
  pipelineStages: [],

  setRecordingPhase: (phase, error) => set({
    recordingPhase: phase,
    recordingError: error ?? null,
  }),
  appendRealtimeSegment: (seg) =>
    set((state) => {
      // 用 segment_id 去重：相同 id 的 segment 替换（partial→final 更新）
      const filtered = state.realtimeSegments.filter(
        (s) => s.segment_id !== seg.segment_id
      );
      return { realtimeSegments: [...filtered, seg] };
    }),
  clearRealtimeSegments: () => set({ realtimeSegments: [] }),
  appendPipelineStage: (stage) =>
    set((state) => ({
      pipelineStages: [...state.pipelineStages, stage],
    })),
  clearPipelineStages: () => set({ pipelineStages: [] }),
}));
