export function formatRelativeTime(value: string): string {
  if (value.includes("今天") || value.includes("昨天")) return value;
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "刚刚";
  const diff = Date.now() - date.getTime();
  if (diff < 60 * 60 * 1000) return `${Math.max(1, Math.floor(diff / 60000))} 分钟前`;
  if (diff < 24 * 60 * 60 * 1000) return `${Math.floor(diff / 3600000)} 小时前`;
  if (diff < 2 * 24 * 60 * 60 * 1000) return "昨天";
  return new Intl.DateTimeFormat("zh-CN", { month: "short", day: "numeric" }).format(date);
}
