import { useEffect, useRef } from "react";
import { useParams, useLocation } from "react-router-dom";
import { save } from "@tauri-apps/plugin-dialog";
import { exportReport } from "@/hooks/useTauriCommands";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import {
  useGetMeeting,
  useGetTranscripts,
  useGetActionItems,
  useTranscribeAudio,
  useRunPipeline,
  useUpdateActionItemStatus,
  useListMeetings,
} from "@/hooks/useTauriCommands";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { RecordButton } from "@/components/RecordButton";
import { TranscriptView } from "@/components/TranscriptView";
import { ActionItemList } from "@/components/ActionItemList";
import { useMeetingStore } from "@/store/meetingStore";
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
  } = useMeetingStore();

  const { isRecording, error, startRecording, stopRecording } = useRecording(meetingId);
  const getMeeting = useGetMeeting();
  const getTranscripts = useGetTranscripts();
  const getActionItems = useGetActionItems();
  const transcribeAudio = useTranscribeAudio();
  const runPipeline = useRunPipeline();
  const updateActionItemStatus = useUpdateActionItemStatus();
  const listMeetings = useListMeetings();

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
      startRecording();
    }
  }, [currentMeeting, startRecording]);

  async function handleStopAndProcess() {
    const audioPath = await stopRecording();
    if (!audioPath || !meetingId) return;
    setCurrentMeetingStatus("processing");
    try {
      await transcribeAudio(audioPath, meetingId);
      await loadTranscripts();
      await runPipeline(meetingId);
      await loadMeeting();
      await loadActionItems();
      const updatedMeetings = await listMeetings();
      setMeetings(updatedMeetings);
      setCurrentMeetingStatus("completed");
    } catch (e) {
      console.error("Processing failed:", e);
      setCurrentMeetingStatus("error");
    }
  }

  async function handleToggleActionItem(itemId: number, status: "pending" | "done") {
    await updateActionItemStatus(itemId, status);
    await loadActionItems();
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
        <RecordButton
          isRecording={isRecording}
          disabled={currentMeeting.status === "processing"}
          onStart={startRecording}
          onStop={handleStopAndProcess}
        />
        {currentMeeting.status === "processing" && (
          <p className="text-sm text-amber-500 font-medium">{t("meeting.processingNotice")}</p>
        )}
        {error && (
          <p className="text-sm text-destructive">{error}</p>
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
          {currentMeeting.summary ? (
            <div className="rounded-xl border border-border bg-card p-4 text-sm leading-relaxed whitespace-pre-wrap text-foreground">
              {currentMeeting.summary}
            </div>
          ) : (
            <div className="flex items-center justify-center py-12 text-sm text-muted-foreground">
              {t("meeting.noSummary")}
            </div>
          )}
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
            <div className="rounded-xl border border-border bg-card p-4 text-sm leading-relaxed whitespace-pre-wrap font-mono text-foreground">
              {currentMeeting.report}
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
