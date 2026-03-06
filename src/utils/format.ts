import dayjs from "dayjs";
import duration from "dayjs/plugin/duration";
import relativeTime from "dayjs/plugin/relativeTime";

dayjs.extend(duration);
dayjs.extend(relativeTime);

export function formatDateTime(isoString: string): string {
  return dayjs(isoString).format("YYYY-MM-DD HH:mm");
}

export function formatDuration(startIso: string, endIso: string | null): string {
  if (!endIso) return "进行中";
  const ms = dayjs(endIso).diff(dayjs(startIso));
  const d = dayjs.duration(ms);
  const h = d.hours();
  const m = d.minutes();
  const s = d.seconds();
  if (h > 0) return `${h}h ${m}m`;
  if (m > 0) return `${m}m ${s}s`;
  return `${s}s`;
}

export function formatTimestamp(seconds: number): string {
  const m = Math.floor(seconds / 60);
  const s = Math.floor(seconds % 60);
  return `${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
}

export function truncate(text: string, maxLength: number): string {
  if (text.length <= maxLength) return text;
  return text.slice(0, maxLength) + "...";
}
