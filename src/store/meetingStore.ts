import { create } from "zustand";
import type { Meeting, Transcript, ActionItem, MeetingStatus } from "../types";

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
  setTranscripts: (transcripts: Transcript[]) => void;
  appendTranscript: (transcript: Transcript) => void;
  setActionItems: (items: ActionItem[]) => void;
  setLoading: (loading: boolean) => void;
  setError: (error: string | null) => void;
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
  setTranscripts: (transcripts) => set({ transcripts }),
  appendTranscript: (transcript) =>
    set((state) => ({ transcripts: [...state.transcripts, transcript] })),
  setActionItems: (actionItems) => set({ actionItems }),
  setLoading: (isLoading) => set({ isLoading }),
  setError: (error) => set({ error }),
}));
