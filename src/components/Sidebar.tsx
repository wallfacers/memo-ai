import React, { useEffect, useState } from "react";
import { useNavigate, useLocation } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import { Mic, Settings, Plus } from "lucide-react";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Button } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator";
import { useMeetingStore } from "@/store/meetingStore";
import { cn } from "@/lib/utils";
import type { Meeting } from "@/types";
import { formatDateTime } from "@/utils/format";

const statusDot: Record<Meeting["status"], string> = {
  idle: "bg-muted-foreground/40",
  recording: "bg-destructive animate-pulse",
  processing: "bg-amber-500 animate-pulse",
  completed: "bg-emerald-500",
  error: "bg-destructive",
};

export function Sidebar() {
  const navigate = useNavigate();
  const location = useLocation();
  const { meetings, setMeetings, setError } = useMeetingStore();
  const [newTitle, setNewTitle] = useState("");
  const [creating, setCreating] = useState(false);

  useEffect(() => {
    invoke<Meeting[]>("list_meetings")
      .then(setMeetings)
      .catch((e) => setError(String(e)));
  }, []);

  async function createMeeting() {
    const title = newTitle.trim() || `会议 ${new Date().toLocaleString("zh-CN")}`;
    try {
      setCreating(true);
      const meeting = await invoke<Meeting>("create_meeting", { title });
      setMeetings([meeting, ...meetings]);
      setNewTitle("");
      navigate(`/meeting/${meeting.id}`);
    } catch (e) {
      setError(String(e));
    } finally {
      setCreating(false);
    }
  }

  function currentMeetingId(): number | null {
    const match = location.pathname.match(/\/meeting\/(\d+)/);
    return match ? parseInt(match[1]) : null;
  }

  const activeMeetingId = currentMeetingId();

  return (
    <aside
      className="w-60 shrink-0 flex flex-col border-r border-border h-full"
      style={{ background: "var(--sidebar-background)" }}
    >
      {/* Logo */}
      <div className="flex items-center gap-2 px-4 py-4">
        <div className="flex h-7 w-7 items-center justify-center rounded-lg bg-primary text-primary-foreground">
          <Mic className="h-4 w-4" />
        </div>
        <div>
          <p className="text-sm font-semibold leading-none">Memo AI</p>
          <p className="text-[10px] text-muted-foreground leading-none mt-0.5">AI 会议助手</p>
        </div>
      </div>

      <Separator />

      {/* New meeting input */}
      <div className="px-3 py-2 flex gap-1.5">
        <input
          type="text"
          value={newTitle}
          onChange={(e) => setNewTitle(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && createMeeting()}
          placeholder="会议标题…"
          className="flex-1 min-w-0 text-xs px-2 py-1.5 rounded-md border border-input bg-background placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring"
        />
        <Button
          size="icon"
          variant="outline"
          className="h-7 w-7 shrink-0"
          onClick={createMeeting}
          disabled={creating}
          title="新建会议"
        >
          <Plus className="h-3.5 w-3.5" />
        </Button>
      </div>

      {/* Meeting list */}
      <ScrollArea className="flex-1 px-2">
        <div className="space-y-0.5 py-1">
          {meetings.length === 0 ? (
            <p className="text-xs text-muted-foreground text-center py-6">暂无会议记录</p>
          ) : (
            meetings.map((m) => (
              <button
                key={m.id}
                onClick={() => navigate(`/meeting/${m.id}`)}
                className={cn(
                  "w-full text-left rounded-md px-2 py-2 text-xs transition-colors",
                  "hover:bg-accent hover:text-accent-foreground",
                  activeMeetingId === m.id
                    ? "bg-accent text-accent-foreground font-medium"
                    : "text-foreground/80"
                )}
              >
                <div className="flex items-center gap-1.5 mb-0.5">
                  <span
                    className={cn("h-1.5 w-1.5 rounded-full shrink-0", statusDot[m.status])}
                  />
                  <span className="truncate font-medium">{m.title}</span>
                </div>
                <p className="text-[10px] text-muted-foreground pl-3">
                  {formatDateTime(m.start_time)}
                </p>
              </button>
            ))
          )}
        </div>
      </ScrollArea>

      <Separator />

      {/* Settings */}
      <div className="px-2 py-2">
        <button
          onClick={() => navigate("/settings")}
          className={cn(
            "w-full flex items-center gap-2 rounded-md px-2 py-2 text-xs transition-colors",
            "hover:bg-accent hover:text-accent-foreground",
            location.pathname === "/settings"
              ? "bg-accent text-accent-foreground font-medium"
              : "text-muted-foreground"
          )}
        >
          <Settings className="h-3.5 w-3.5" />
          设置
        </button>
      </div>
    </aside>
  );
}
