export type MeetingStatus = "idle" | "recording" | "processing" | "completed" | "error";

export interface Meeting {
  id: number;
  title: string;
  start_time: string;
  end_time: string | null;
  status: MeetingStatus;
  summary: string | null;
  report: string | null;
  audio_path: string | null;
  created_at: string;
  updated_at: string;
}

export interface Transcript {
  id: number;
  meeting_id: number;
  speaker: string | null;
  text: string;
  timestamp: number;
  confidence: number | null;
  created_at: string;
}

export interface ActionItem {
  id: number;
  meeting_id: number;
  task: string;
  owner: string | null;
  deadline: string | null;
  status: "pending" | "done";
  created_at: string;
}

export interface MeetingStructure {
  id: number;
  meeting_id: number;
  topic: string | null;
  participants: string[];
  key_points: string[];
  decisions: string[];
  risks: string[];
  created_at: string;
}

export interface LlmProvider {
  type: "ollama" | "openai";
  base_url: string;
  model: string;
  api_key: string | null;
}

export interface AppSettings {
  llm_provider: LlmProvider;
  whisper_model: string;
  language: string;
}

export interface PipelineResult {
  clean_transcript: string;
  structure: MeetingStructure;
  summary: string;
  action_items: ActionItem[];
  report: string;
}
