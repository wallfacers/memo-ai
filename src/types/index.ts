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
  auto_titled: boolean;
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

export type AsrProviderType = "local_whisper" | "aliyun" | "funasr";

export interface AppSettings {
  llm_provider: LlmProvider;
  whisper_model: string;
  language: string;
  whisper_cli_path: string;
  whisper_model_dir: string;
  asr_provider: AsrProviderType;
  aliyun_asr_app_key: string;
  aliyun_asr_access_key_id: string;
  aliyun_asr_access_key_secret: string;
  funasr_enabled: boolean;
  funasr_ws_url: string;
  funasr_server_path: string;
  funasr_port: number;
}

export type RecordingPhase =
  | "idle"
  | "connecting"
  | "recording"
  | "stopping"
  | "batch_transcribing"
  | "merging"
  | "pipeline"
  | "done"
  | "error";

export interface StreamingSegment {
  text: string;
  is_final: boolean;
  timestamp_ms: number;
}

export interface PipelineStageDoneEvent {
  stage: number;
  name: string;
  elapsed_ms: number;
}

export interface PipelineStageFailed {
  stage: number;
  error: string;
}

export interface PipelineResult {
  clean_transcript: string;
  summary: string;
  report: string;
  generated_title?: string;
}
