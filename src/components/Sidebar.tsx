import { useEffect, useState, useRef, useCallback } from "react";
import { useNavigate, useLocation } from "react-router-dom";
import { Mic, Settings, Plus, Search, Pencil, Trash2, Check, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import {
  useListMeetings,
  useCreateMeeting,
  useDeleteMeeting,
  useRenameMeeting,
  searchMeetings,
} from "@/hooks/useTauriCommands";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Separator } from "@/components/ui/separator";
import { useMeetingStore } from "@/store/meetingStore";
import { cn } from "@/lib/utils";
import type { Meeting } from "@/types";
import { formatDateTime, formatDuration } from "@/utils/format";

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
  const { t } = useTranslation();
  const { meetings, setMeetings, setError } = useMeetingStore();
  const [creating, setCreating] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [searchResults, setSearchResults] = useState<Meeting[] | null>(null);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const [renamingId, setRenamingId] = useState<number | null>(null);
  const [renameValue, setRenameValue] = useState("");
  const renameInputRef = useRef<HTMLInputElement>(null);

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
  const deleteMeetingCmd = useDeleteMeeting();
  const renameMeetingCmd = useRenameMeeting();

  useEffect(() => {
    listMeetings()
      .then(setMeetings)
      .catch((e) => setError(String(e)));
  }, [listMeetings, setMeetings, setError]);

  useEffect(() => {
    if (renamingId !== null) {
      setTimeout(() => renameInputRef.current?.focus(), 0);
    }
  }, [renamingId]);

  async function createMeeting() {
    const now = new Date();
    const title = `${t("meeting.meetingPrefix")} ${now.getFullYear()}-${String(now.getMonth()+1).padStart(2,"0")}-${String(now.getDate()).padStart(2,"0")} ${String(now.getHours()).padStart(2,"0")}:${String(now.getMinutes()).padStart(2,"0")}:${String(now.getSeconds()).padStart(2,"0")}`;
    try {
      setCreating(true);
      const meeting = await createMeetingCmd(title, true);
      setMeetings([meeting, ...useMeetingStore.getState().meetings]);
      navigate(`/meeting/${meeting.id}`);
    } catch (e) {
      setError(String(e));
    } finally {
      setCreating(false);
    }
  }

  async function deleteMeeting(id: number, e: React.MouseEvent) {
    e.stopPropagation();
    try {
      await deleteMeetingCmd(id);
      const updated = useMeetingStore.getState().meetings.filter((m) => m.id !== id);
      setMeetings(updated);
      if (searchResults) setSearchResults(searchResults.filter((m) => m.id !== id));
      if (location.pathname === `/meeting/${id}`) navigate("/");
    } catch (e) {
      setError(String(e));
    }
  }

  function startRename(m: Meeting, e: React.MouseEvent) {
    e.stopPropagation();
    setRenamingId(m.id);
    setRenameValue(m.title);
  }

  async function confirmRename(id: number) {
    const trimmed = renameValue.trim();
    if (trimmed) {
      try {
        await renameMeetingCmd(id, trimmed);
        setMeetings(
          useMeetingStore.getState().meetings.map((m) =>
            m.id === id ? { ...m, title: trimmed } : m
          )
        );
        if (searchResults) {
          setSearchResults(
            searchResults.map((m) => (m.id === id ? { ...m, title: trimmed } : m))
          );
        }
      } catch (e) {
        setError(String(e));
      }
    }
    setRenamingId(null);
  }

  function cancelRename() {
    setRenamingId(null);
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
      {/* Logo + 新建按钮 */}
      <div className="flex items-center justify-between px-4 py-4">
        <button
          onClick={() => navigate("/")}
          className="flex items-center gap-2 hover:opacity-80 transition-opacity"
        >
          <div className="flex h-7 w-7 items-center justify-center rounded-lg bg-primary text-primary-foreground">
            <Mic className="h-4 w-4" />
          </div>
          <div>
            <p className="text-sm font-semibold leading-none">Memo AI</p>
            <p className="text-[10px] text-muted-foreground leading-none mt-0.5">{t("sidebar.appSubtitle")}</p>
          </div>
        </button>
        <button
          onClick={createMeeting}
          disabled={creating}
          title={t("sidebar.newMeeting")}
          className="flex h-7 w-7 items-center justify-center rounded-md border border-input bg-background hover:bg-accent hover:text-accent-foreground transition-colors disabled:opacity-50"
        >
          <Plus className="h-3.5 w-3.5" />
        </button>
      </div>

      <Separator />

      {/* 搜索框 */}
      <div className="px-3 pt-2 pb-1">
        <div className="relative">
          <Search className="absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground" />
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => handleSearch(e.target.value)}
            placeholder={t("sidebar.searchPlaceholder")}
            className="w-full rounded-md border border-input bg-background py-1.5 pl-8 pr-3 text-xs placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring"
          />
        </div>
      </div>

      {/* Meeting list */}
      <ScrollArea className="flex-1 px-2">
        <div className="space-y-0.5 py-1">
          {(() => {
            const displayedMeetings = searchResults ?? meetings;
            if (displayedMeetings.length === 0) {
              return (
                <p className="text-xs text-muted-foreground text-center py-6">
                  {searchQuery ? t("sidebar.noResults") : t("sidebar.noMeetings")}
                </p>
              );
            }
            return displayedMeetings.map((m) => (
              <div
                key={m.id}
                className={cn(
                  "group relative rounded-md transition-colors",
                  activeMeetingId === m.id
                    ? "bg-accent text-accent-foreground"
                    : "hover:bg-accent/60"
                )}
              >
                {renamingId === m.id ? (
                  <div className="flex items-center gap-1 px-2 py-1.5">
                    <input
                      ref={renameInputRef}
                      value={renameValue}
                      onChange={(e) => setRenameValue(e.target.value)}
                      onKeyDown={(e) => {
                        if (e.key === "Enter") confirmRename(m.id);
                        if (e.key === "Escape") cancelRename();
                      }}
                      className="flex-1 min-w-0 text-xs px-1.5 py-0.5 rounded border border-ring bg-background focus:outline-none"
                    />
                    <button
                      onClick={() => confirmRename(m.id)}
                      className="text-emerald-500 hover:text-emerald-600"
                    >
                      <Check className="h-3.5 w-3.5" />
                    </button>
                    <button
                      onClick={cancelRename}
                      className="text-muted-foreground hover:text-foreground"
                    >
                      <X className="h-3.5 w-3.5" />
                    </button>
                  </div>
                ) : (
                  <button
                    onClick={() => navigate(`/meeting/${m.id}`)}
                    className="w-full text-left px-2 py-2 text-xs"
                  >
                    <div className="flex items-center gap-1.5 mb-0.5 pr-10">
                      <span
                        className={cn("h-1.5 w-1.5 rounded-full shrink-0", statusDot[m.status])}
                      />
                      <span
                        className={cn(
                          "truncate font-medium",
                          activeMeetingId === m.id ? "text-accent-foreground" : "text-foreground/80"
                        )}
                      >
                        {m.title}
                      </span>
                    </div>
                    <div className="flex items-center gap-2 pl-3 text-[10px] text-muted-foreground">
                      <span>{formatDateTime(m.start_time)}</span>
                      {m.end_time && (
                        <span className="shrink-0">{formatDuration(m.start_time, m.end_time)}</span>
                      )}
                    </div>
                  </button>
                )}

                {renamingId !== m.id && (
                  <div className="absolute right-1.5 top-1/2 -translate-y-1/2 hidden group-hover:flex items-center gap-0.5">
                    <button
                      onClick={(e) => startRename(m, e)}
                      title={t("sidebar.rename")}
                      className="p-1 rounded text-muted-foreground hover:text-foreground hover:bg-background/80 transition-colors"
                    >
                      <Pencil className="h-3 w-3" />
                    </button>
                    <button
                      onClick={(e) => deleteMeeting(m.id, e)}
                      title={t("sidebar.delete")}
                      className="p-1 rounded text-muted-foreground hover:text-destructive hover:bg-background/80 transition-colors"
                    >
                      <Trash2 className="h-3 w-3" />
                    </button>
                  </div>
                )}
              </div>
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
          {t("sidebar.settings")}
        </button>
      </div>
    </aside>
  );
}
