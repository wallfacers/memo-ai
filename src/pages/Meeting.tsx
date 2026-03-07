import { useEffect, useRef } from "react";
import { useParams, useLocation } from "react-router-dom";
import { save } from "@tauri-apps/plugin-dialog";
import { exportReport } from "@/hooks/useTauriCommands";
import { useTranslation } from "react-i18next";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { Badge } from "@/components/ui/badge";
import { Loader2, CheckCircle2, XCircle } from "lucide-react";
import {
  useGetMeeting,
  useGetTranscripts,
  useGetActionItems,
  useTranscribeAudio,
  useRunPipeline,
  useUpdateActionItemStatus,
  useListMeetings,
  useStartFunAsrSession,
  useStopFunAsrSession,
} from "@/hooks/useTauriCommands";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { RecordButton } from "@/components/RecordButton";
import { TranscriptView } from "@/components/TranscriptView";
import { ActionItemList } from "@/components/ActionItemList";
import { RealtimeTranscript } from "@/components/RealtimeTranscript";
import { PipelineProgress } from "@/components/PipelineProgress";
import { SummaryTab } from "@/components/SummaryTab";
import { useMeetingStore } from "@/store/meetingStore";
import { useSettingsStore } from "@/store/settingsStore";
import { useRecording } from "@/hooks/useRecording";
import type { Meeting as MeetingType } from "@/types";

type StatusVariant = "default" | "destructive" | "secondary" | "outline";
const statusVariant: Record<MeetingType["status"], StatusVariant> = {
  idle: "secondary",
  recording: "destructive",
  processing: "outline",
  completed: "default",
  error: "destructive",
};

export function Meeting() {
  const { id } = useParams<{ id: string }>();
  const meetingId = id ? parseInt(id) : null;
  const location = useLocation();
  const autoRecordRef = useRef(location.state?.autoRecord === true);
  const { t } = useTranslation();

  const {
    currentMeeting,
    setCurrentMeeting,
    transcripts,
    setTranscripts,
    actionItems,
    setActionItems,
    setCurrentMeetingStatus,
    setMeetings,
    recordingPhase,
    setRecordingPhase,
    clearRealtimeSegments,
    clearPipelineStages,
  } = useMeetingStore();

  const { settings } = useSettingsStore();
  const { isRecording, error, startRecording, stopRecording } = useRecording(meetingId);
  const getMeeting = useGetMeeting();
  const getTranscripts = useGetTranscripts();
  const getActionItems = useGetActionItems();
  const transcribeAudio = useTranscribeAudio();
  const runPipeline = useRunPipeline();
  const updateActionItemStatus = useUpdateActionItemStatus();
  const listMeetings = useListMeetings();
  const startFunAsrSession = useStartFunAsrSession();
  const stopFunAsrSession = useStopFunAsrSession();

  async function loadMeeting() {
    const meeting = await getMeeting(meetingId!);
    setCurrentMeeting(meeting);
  }

  async function loadTranscripts() {
    const data = await getTranscripts(meetingId!);
    setTranscripts(data);
  }

  async function loadActionItems() {
    const data = await getActionItems(meetingId!);
    setActionItems(data);
  }

  useEffect(() => {
    if (!meetingId) return;

    async function fetchMeeting() {
      const meeting = await getMeeting(meetingId!);
      setCurrentMeeting(meeting);
    }
    async function fetchTranscripts() {
      const data = await getTranscripts(meetingId!);
      setTranscripts(data);
    }
    async function fetchActionItems() {
      const data = await getActionItems(meetingId!);
      setActionItems(data);
    }

    void fetchMeeting();
    void fetchTranscripts();
    void fetchActionItems();
  }, [meetingId, getMeeting, getTranscripts, getActionItems, setCurrentMeeting, setTranscripts, setActionItems]);

  useEffect(() => {
    if (autoRecordRef.current && currentMeeting?.status === "idle") {
      autoRecordRef.current = false;
      void handleStartRecording();
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [currentMeeting]);

  async function handleStartRecording() {
    setRecordingPhase("connecting");
    clearRealtimeSegments();
    clearPipelineStages();
    try {
      if (settings.funasr_enabled) {
        await startFunAsrSession(meetingId!);
      }
      await startRecording();
      setRecordingPhase("recording");
    } catch (e) {
      console.error("Start recording failed:", e);
      setRecordingPhase("error");
    }
  }

  async function handleStopRecording() {
    setRecordingPhase("stopping");
    try {
      if (settings.funasr_enabled) {
        await stopFunAsrSession();
      }

      const audioPath = await stopRecording();
      if (!audioPath || !meetingId) {
        setRecordingPhase("error");
        return;
      }

      setCurrentMeetingStatus("processing");
      setRecordingPhase("batch_transcribing");
      await transcribeAudio(audioPath, meetingId);
      await loadTranscripts();

      setRecordingPhase("merging");
      await new Promise((r) => setTimeout(r, 500));

      setRecordingPhase("pipeline");
      await runPipeline(meetingId);

      setRecordingPhase("done");
      await loadMeeting();
      await loadActionItems();
      const updatedMeetings = await listMeetings();
      setMeetings(updatedMeetings);
      setCurrentMeetingStatus("completed");
    } catch (e) {
      console.error("Processing failed:", e);
      setRecordingPhase("error");
      setCurrentMeetingStatus("error");
    }
  }

  async function handleToggleActionItem(itemId: number, status: "pending" | "done") {
    await updateActionItemStatus(itemId, status);
    await loadActionItems();
  }

  function handleSummaryUpdated(newSummary: string) {
    if (currentMeeting) {
      setCurrentMeeting({ ...currentMeeting, summary: newSummary });
    }
  }

  if (!currentMeeting) {
    return (
      <div className="flex h-full items-center justify-center text-sm text-muted-foreground">
        {t("meeting.loading")}
      </div>
    );
  }

  return (
    <div className="flex flex-col flex-1 min-h-0">
      {/* Header */}
      <div className="flex items-center justify-between border-b border-border px-6 py-4 shrink-0">
        <h2 className="text-lg font-semibold text-foreground truncate pr-4">
          {currentMeeting.title}
        </h2>
        <Badge variant={statusVariant[currentMeeting.status]} className="shrink-0">
          {t(`meeting.status.${currentMeeting.status}`)}
        </Badge>
      </div>

      {/* Record area */}
      <div className="flex flex-col items-center gap-3 py-6 border-b border-border shrink-0">
        {/* 录音阶段状态栏 */}
        {recordingPhase !== "idle" && (
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            {recordingPhase === "connecting" && <><Loader2 className="h-3.5 w-3.5 animate-spin" /><span>{t("meeting.phase.connecting")}</span></>}
            {recordingPhase === "recording" && <><span className="h-2 w-2 rounded-full bg-red-500 animate-pulse" /><span>{t("meeting.phase.recording")}</span></>}
            {recordingPhase === "stopping" && <><Loader2 className="h-3.5 w-3.5 animate-spin" /><span>{t("meeting.phase.stopping")}</span></>}
            {recordingPhase === "batch_transcribing" && <><Loader2 className="h-3.5 w-3.5 animate-spin" /><span>{t("meeting.phase.batchTranscribing")}</span></>}
            {recordingPhase === "merging" && <><Loader2 className="h-3.5 w-3.5 animate-spin" /><span>{t("meeting.phase.merging")}</span></>}
            {recordingPhase === "pipeline" && <><Loader2 className="h-3.5 w-3.5 animate-spin" /><span>{t("meeting.phase.pipeline")}</span></>}
            {recordingPhase === "done" && <><CheckCircle2 className="h-3.5 w-3.5 text-green-600" /><span>{t("meeting.phase.done")}</span></>}
            {recordingPhase === "error" && <><XCircle className="h-3.5 w-3.5 text-destructive" /><span>{t("meeting.phase.error")}</span></>}
          </div>
        )}

        <RecordButton
          isRecording={isRecording}
          disabled={currentMeeting.status === "processing"}
          onStart={handleStartRecording}
          onStop={handleStopRecording}
        />

        {currentMeeting.status === "processing" && (
          <p className="text-sm text-amber-500 font-medium">{t("meeting.processingNotice")}</p>
        )}
        {error && (
          <p className="text-sm text-destructive">{error}</p>
        )}

        {/* 录音时实时字幕 */}
        {(recordingPhase === "recording" || recordingPhase === "stopping") && settings.funasr_enabled && (
          <RealtimeTranscript />
        )}

        {/* Pipeline 进度 */}
        {(recordingPhase === "pipeline" || recordingPhase === "done") && (
          <PipelineProgress />
        )}
      </div>

      {/* Tabs */}
      <Tabs defaultValue="transcript" className="flex-1 flex flex-col overflow-hidden min-h-0 px-6 pt-4">
        <TabsList className="shrink-0">
          <TabsTrigger value="transcript">{t("meeting.tabs.transcript")}</TabsTrigger>
          <TabsTrigger value="actions">{t("meeting.tabs.actions")}</TabsTrigger>
          <TabsTrigger value="summary">{t("meeting.tabs.summary")}</TabsTrigger>
          <TabsTrigger value="report">{t("meeting.tabs.report")}</TabsTrigger>
        </TabsList>

        <TabsContent value="transcript" className="flex-1 overflow-auto min-h-0 mt-4">
          <TranscriptView transcripts={transcripts} />
        </TabsContent>

        <TabsContent value="actions" className="flex-1 overflow-auto mt-4">
          <ActionItemList items={actionItems} onToggle={handleToggleActionItem} />
        </TabsContent>

        <TabsContent value="summary" className="flex-1 overflow-auto mt-4">
          <SummaryTab
            meeting={currentMeeting}
            onSummaryUpdated={handleSummaryUpdated}
          />
        </TabsContent>

        <TabsContent value="report" className="flex-1 overflow-auto mt-4">
          <div className="mb-3 flex justify-end">
            <button
              onClick={async () => {
                const filePath = await save({
                  defaultPath: `${currentMeeting.title ?? "report"}.md`,
                  filters: [{ name: "Markdown", extensions: ["md"] }],
                });
                if (filePath) {
                  try {
                    await exportReport(currentMeeting.id, filePath);
                  } catch (e) {
                    console.error("Export failed:", e);
                  }
                }
              }}
              className="rounded-md bg-primary px-3 py-1.5 text-sm font-medium text-primary-foreground hover:bg-primary/90"
            >
              {t("meeting.exportMd")}
            </button>
          </div>
          {currentMeeting.report ? (
            <div className="rounded-xl border border-border bg-card p-4 text-sm leading-relaxed text-foreground prose prose-sm max-w-none dark:prose-invert">
              <ReactMarkdown remarkPlugins={[remarkGfm]}>
                {currentMeeting.report}
              </ReactMarkdown>
            </div>
          ) : (
            <div className="flex items-center justify-center py-12 text-sm text-muted-foreground">
              {t("meeting.noReport")}
            </div>
          )}
        </TabsContent>
      </Tabs>
    </div>
  );
}
