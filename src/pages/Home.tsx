import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { Mic } from "lucide-react";
import { useCreateMeeting } from "@/hooks/useTauriCommands";
import { useMeetingStore } from "@/store/meetingStore";

export function Home() {
  const navigate = useNavigate();
  const createMeeting = useCreateMeeting();
  const { setMeetings } = useMeetingStore();
  const [creating, setCreating] = useState(false);

  async function handleQuickStart() {
    if (creating) return;
    setCreating(true);
    try {
      const now = new Date();
      const title = `会议 ${now.getFullYear()}-${String(now.getMonth()+1).padStart(2,"0")}-${String(now.getDate()).padStart(2,"0")} ${String(now.getHours()).padStart(2,"0")}:${String(now.getMinutes()).padStart(2,"0")}:${String(now.getSeconds()).padStart(2,"0")}`;
      const meeting = await createMeeting(title, true);
      setMeetings([meeting, ...useMeetingStore.getState().meetings]);
      navigate(`/meeting/${meeting.id}`, { state: { autoRecord: true } });
    } finally {
      setCreating(false);
    }
  }

  return (
    <div className="flex flex-1 flex-col items-center justify-center gap-6 text-center px-8">
      <button
        onClick={handleQuickStart}
        disabled={creating}
        className="flex h-24 w-24 items-center justify-center rounded-full bg-primary text-primary-foreground shadow-lg transition-all duration-200 hover:bg-primary/90 hover:shadow-xl active:scale-95 disabled:opacity-60 disabled:cursor-not-allowed"
      >
        <Mic className="h-10 w-10" />
      </button>
      <div>
        <h2 className="text-xl font-semibold text-foreground">
          {creating ? "正在创建…" : "点击开始录音"}
        </h2>
        <p className="mt-1.5 text-sm text-muted-foreground">
          将自动创建新会议并开始录制
        </p>
      </div>
    </div>
  );
}
