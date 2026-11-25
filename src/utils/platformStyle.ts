export interface PlatformVisual {
  mark: string;
  color: string;
  badgeClass: string;
  avatarClass: string;
  barColor: string;
}

const PLATFORM_VISUALS: Record<string, PlatformVisual> = {
  掘金: {
    mark: "稀",
    color: "#1E80FF",
    badgeClass: "border-[#1E80FF]/20 bg-[#1E80FF]/8 text-[#1565D8]",
    avatarClass: "bg-[#1E80FF]/12 text-[#1565D8]",
    barColor: "#1E80FF",
  },
  Bilibili: {
    mark: "B",
    color: "#FB7299",
    badgeClass: "border-[#FB7299]/25 bg-[#FB7299]/10 text-[#D94A76]",
    avatarClass: "bg-[#FB7299]/14 text-[#D94A76]",
    barColor: "#FB7299",
  },
  知乎: {
    mark: "知",
    color: "#0066FF",
    badgeClass: "border-[#0066FF]/20 bg-[#0066FF]/8 text-[#0052CC]",
    avatarClass: "bg-[#0066FF]/12 text-[#0052CC]",
    barColor: "#0066FF",
  },
  CSDN: {
    mark: "C",
    color: "#E05A2B",
    badgeClass: "border-[#E05A2B]/20 bg-[#E05A2B]/10 text-[#C2451A]",
    avatarClass: "bg-[#E05A2B]/12 text-[#C2451A]",
    barColor: "#E05A2B",
  },
  微信公众号: {
    mark: "微",
    color: "#07C160",
    badgeClass: "border-[#07C160]/25 bg-[#07C160]/10 text-[#059449]",
    avatarClass: "bg-[#07C160]/12 text-[#059449]",
    barColor: "#07C160",
  },
  YouTube: {
    mark: "Y",
    color: "#E53935",
    badgeClass: "border-[#E53935]/20 bg-[#E53935]/10 text-[#C62828]",
    avatarClass: "bg-[#E53935]/12 text-[#C62828]",
    barColor: "#E53935",
  },
  小红书: {
    mark: "红",
    color: "#FF2442",
    badgeClass: "border-[#FF2442]/20 bg-[#FF2442]/10 text-[#D81B36]",
    avatarClass: "bg-[#FF2442]/12 text-[#D81B36]",
    barColor: "#FF2442",
  },
  抖音: {
    mark: "抖",
    color: "#1F2937",
    badgeClass: "border-stone-300 bg-stone-100 text-stone-700",
    avatarClass: "bg-stone-200/80 text-stone-800",
    barColor: "#2F4A36",
  },
  快手: {
    mark: "快",
    color: "#FF5000",
    badgeClass: "border-[#FF5000]/20 bg-[#FF5000]/10 text-[#D94300]",
    avatarClass: "bg-[#FF5000]/12 text-[#D94300]",
    barColor: "#FF5000",
  },
  微博: {
    mark: "博",
    color: "#E6A23C",
    badgeClass: "border-[#E6A23C]/25 bg-[#E6A23C]/12 text-[#B87D1E]",
    avatarClass: "bg-[#E6A23C]/14 text-[#B87D1E]",
    barColor: "#E6A23C",
  },
};

export function getPlatformVisual(platformName: string): PlatformVisual {
  return (
    PLATFORM_VISUALS[platformName] ?? {
      mark: platformName.slice(0, 1).toUpperCase() || "P",
      color: "#2F4A36",
      badgeClass: "border-[#2F4A36]/20 bg-[#EAF0EA] text-[#2F4A36]",
      avatarClass: "bg-[#EAF0EA] text-[#2F4A36]",
      barColor: "#2F4A36",
    }
  );
}
