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
    color: "#6B9ED4",
    badgeClass: "border-[#C4DDF0] bg-[#EAF3FA] text-[#2D5F8A]",
    avatarClass: "bg-[#EAF3FA] text-[#2D5F8A]",
    barColor: "#6B9ED4",
  },
  Bilibili: {
    mark: "B",
    color: "#E38AA6",
    badgeClass: "border-[#F3CBD8] bg-[#FCECF1] text-[#A84467]",
    avatarClass: "bg-[#FCECF1] text-[#A84467]",
    barColor: "#E38AA6",
  },
  知乎: {
    mark: "知",
    color: "#8793D6",
    badgeClass: "border-[#D5DAF4] bg-[#EEF0FC] text-[#455094]",
    avatarClass: "bg-[#EEF0FC] text-[#455094]",
    barColor: "#8793D6",
  },
  CSDN: {
    mark: "C",
    color: "#E39472",
    badgeClass: "border-[#F3D1C2] bg-[#FDF1EB] text-[#A65232]",
    avatarClass: "bg-[#FDF1EB] text-[#A65232]",
    barColor: "#E39472",
  },
  微信公众号: {
    mark: "微",
    color: "#68B692",
    badgeClass: "border-[#BEE0CF] bg-[#E7F4EE] text-[#286B51]",
    avatarClass: "bg-[#E7F4EE] text-[#286B51]",
    barColor: "#68B692",
  },
  YouTube: {
    mark: "Y",
    color: "#DC7A82",
    badgeClass: "border-[#F3C4C9] bg-[#FDECEE] text-[#A63D47]",
    avatarClass: "bg-[#FDECEE] text-[#A63D47]",
    barColor: "#DC7A82",
  },
  小红书: {
    mark: "红",
    color: "#E57A8E",
    badgeClass: "border-[#F4C4CE] bg-[#FDEBEF] text-[#AD3850]",
    avatarClass: "bg-[#FDEBEF] text-[#AD3850]",
    barColor: "#E57A8E",
  },
  抖音: {
    mark: "抖",
    color: "#8A7DA3",
    badgeClass: "border-[#DDD5EA] bg-[#F3EFF8] text-[#53466E]",
    avatarClass: "bg-[#F3EFF8] text-[#53466E]",
    barColor: "#8A7DA3",
  },
  快手: {
    mark: "快",
    color: "#E6A06B",
    badgeClass: "border-[#F5D6BC] bg-[#FEF3E8] text-[#A65F28]",
    avatarClass: "bg-[#FEF3E8] text-[#A65F28]",
    barColor: "#E6A06B",
  },
  微博: {
    mark: "博",
    color: "#DFB158",
    badgeClass: "border-[#F2DCA6] bg-[#FEF6E4] text-[#8A6016]",
    avatarClass: "bg-[#FEF6E4] text-[#8A6016]",
    barColor: "#DFB158",
  },
};

export function getPlatformVisual(platformName: string): PlatformVisual {
  return (
    PLATFORM_VISUALS[platformName] ?? {
      mark: platformName.slice(0, 1).toUpperCase() || "P",
      color: "#7B68A8",
      badgeClass: "border-[#DDD2EE] bg-[#F2ECF9] text-[#5B4888]",
      avatarClass: "bg-[#F2ECF9] text-[#5B4888]",
      barColor: "#7B68A8",
    }
  );
}
