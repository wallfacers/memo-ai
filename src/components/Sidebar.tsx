import { useEffect, useState, useRef, useCallback } from "react";
import { useNavigate, useLocation } from "react-router-dom";
import { Mic, Settings, Plus, Search } from "lucide-react";
import { useListMeetings, useCreateMeeting, searchMeetings } from "@/hooks/useTauriCommands";
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
  const [searchQuery, setSearchQuery] = useState("");
  const [searchResults, setSearchResults] = useState<Meeting[] | null>(null);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const handleSearch = useCallback((q: string) => {
    setSearchQuery(q);
    if (debounceRef.current) clearTimeout(debounceRef.current);
    if (!q.trim()) {
      setSearchResults(null);
      return;
    }
    debounceRef.current = setTimeout(async () => {
      try {
        const results = await searchMeetings(q.trim());
        setSearchResults(results);
      } catch (e) {
        console.error("Search failed:", e);
      }
    }, 300);
  }, []);

  useEffect(() => {
    return () => {
      if (debounceRef.current) clearTimeout(debounceRef.current);
    };
  }, []);
  const listMeetings = useListMeetings();
  const createMeetingCmd = useCreateMeeting();

  useEffect(() => {
    listMeetings()
      .then(setMeetings)
      .catch((e) => setError(String(e)));
  }, [listMeetings, setMeetings, setError]);

  async function createMeeting() {
    const autoTitled = newTitle.trim() === "";
    const title = newTitle.trim() || `会议 ${new Date().toLocaleString("zh-CN")}`;
    try {
      setCreating(true);
      const meeting = await createMeetingCmd(title, autoTitled);
      setMeetings([meeting, ...useMeetingStore.getState().meetings]);
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
      <button
        onClick={() => navigate("/")}
        className="flex items-center gap-2 px-4 py-4 w-full text-left hover:opacity-80 transition-opacity"
      >
        <div className="flex h-7 w-7 items-center justify-center rounded-lg bg-primary text-primary-foreground">
          <Mic className="h-4 w-4" />
        </div>
        <div>
          <p className="text-sm font-semibold leading-none">Memo AI</p>
          <p className="text-[10px] text-muted-foreground leading-none mt-0.5">AI 会议助手</p>
        </div>
      </button>

      <Separator />

      {/* 搜索框 */}
      <div className="px-3 pt-2 pb-1">
        <div className="relative">
          <Search className="absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground" />
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => handleSearch(e.target.value)}
            placeholder="搜索会议..."
            className="w-full rounded-md border border-input bg-background py-1.5 pl-8 pr-3 text-xs placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring"
          />
        </div>
      </div>

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
          {(() => {
            const displayedMeetings = searchResults ?? meetings;
            if (displayedMeetings.length === 0) {
              return (
                <p className="text-xs text-muted-foreground text-center py-6">
                  {searchQuery ? "无匹配会议" : "暂无会议记录"}
                </p>
              );
            }
            return displayedMeetings.map((m) => (
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
            ));
          })()}
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
