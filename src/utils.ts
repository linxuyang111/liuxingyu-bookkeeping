/** 分 → 元，带千分位，如 123456 → "1,234.56" */
export function fmtMoney(cents: number): string {
  return (cents / 100).toLocaleString("zh-CN", {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  });
}

export function todayStr(): string {
  const d = new Date();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${d.getFullYear()}-${m}-${day}`;
}

const WEEKDAYS = ["日", "一", "二", "三", "四", "五", "六"];

/** 日期标签：今天 / 昨天 / 2026年11月2日 周一 */
export function dayLabel(dateStr: string): string {
  if (dateStr === todayStr()) return "今天";
  const yesterday = new Date();
  yesterday.setDate(yesterday.getDate() - 1);
  const m = String(yesterday.getMonth() + 1).padStart(2, "0");
  const day = String(yesterday.getDate()).padStart(2, "0");
  if (dateStr === `${yesterday.getFullYear()}-${m}-${day}`) return "昨天";
  const [y, mo, da] = dateStr.split("-").map(Number);
  const d = new Date(y, mo - 1, da);
  return `${y}年${mo}月${da}日 周${WEEKDAYS[d.getDay()]}`;
}
