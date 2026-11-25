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
    badgeClass: "border-sky-border bg-sky-bg text-sky-text",
    avatarClass: "bg-sky-bg text-sky-text",
    barColor: "#6B9ED4",
  },
  Bilibili: {
    mark: "B",
    color: "#E38AA6",
    badgeClass: "border-rose-border bg-rose-bg text-rose-text",
    avatarClass: "bg-rose-bg text-rose-text",
    barColor: "#E38AA6",
  },
  知乎: {
    mark: "知",
    color: "#8793D6",
    badgeClass: "border-accent-border bg-accent-soft text-accent-deep",
    avatarClass: "bg-accent-soft text-accent-deep",
    barColor: "#8793D6",
  },
  CSDN: {
    mark: "C",
    color: "#E39472",
    badgeClass: "border-lemon-border bg-lemon-bg text-lemon-text",
    avatarClass: "bg-lemon-bg text-lemon-text",
    barColor: "#E39472",
  },
  微信公众号: {
    mark: "微",
    color: "#68B692",
    badgeClass: "border-mint-border bg-mint-bg text-mint-text",
    avatarClass: "bg-mint-bg text-mint-text",
    barColor: "#68B692",
  },
  YouTube: {
    mark: "Y",
    color: "#DC7A82",
    badgeClass: "border-coral-border bg-coral-bg text-coral-text",
    avatarClass: "bg-coral-bg text-coral-text",
    barColor: "#DC7A82",
  },
  小红书: {
    mark: "红",
    color: "#E57A8E",
    badgeClass: "border-coral-border bg-coral-bg text-coral-text",
    avatarClass: "bg-coral-bg text-coral-text",
    barColor: "#E57A8E",
  },
  抖音: {
    mark: "抖",
    color: "#8A7DA3",
    badgeClass: "border-accent-border bg-accent-soft text-accent-deep",
    avatarClass: "bg-accent-soft text-accent-deep",
    barColor: "#8A7DA3",
  },
  快手: {
    mark: "快",
    color: "#E6A06B",
    badgeClass: "border-lemon-border bg-lemon-bg text-lemon-text",
    avatarClass: "bg-lemon-bg text-lemon-text",
    barColor: "#E6A06B",
  },
  微博: {
    mark: "博",
    color: "#DFB158",
    badgeClass: "border-lemon-border bg-lemon-bg text-lemon-text",
    avatarClass: "bg-lemon-bg text-lemon-text",
    barColor: "#DFB158",
  },
};

export function getPlatformVisual(platformName: string): PlatformVisual {
  return (
    PLATFORM_VISUALS[platformName] ?? {
      mark: platformName.slice(0, 1).toUpperCase() || "P",
      color: "#7B68A8",
      badgeClass: "border-accent-border bg-accent-soft text-accent-deep",
      avatarClass: "bg-accent-soft text-accent-deep",
      barColor: "#7B68A8",
    }
  );
}
