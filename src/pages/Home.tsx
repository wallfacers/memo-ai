import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { Mic } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useCreateMeeting } from "@/hooks/useTauriCommands";
import { useMeetingStore } from "@/store/meetingStore";

export function Home() {
  const navigate = useNavigate();
  const { t } = useTranslation();
  const createMeeting = useCreateMeeting();
  const { setMeetings } = useMeetingStore();
  const [creating, setCreating] = useState(false);

  async function handleQuickStart() {
    if (creating) return;
    setCreating(true);
    try {
      const now = new Date();
      const title = `${t("meeting.meetingPrefix")} ${now.getFullYear()}-${String(now.getMonth()+1).padStart(2,"0")}-${String(now.getDate()).padStart(2,"0")} ${String(now.getHours()).padStart(2,"0")}:${String(now.getMinutes()).padStart(2,"0")}:${String(now.getSeconds()).padStart(2,"0")}`;
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
          {creating ? t("home.creating") : t("home.clickToRecord")}
        </h2>
        <p className="mt-1.5 text-sm text-muted-foreground">
          {t("home.autoCreate")}
        </p>
      </div>
    </div>
  );
}
