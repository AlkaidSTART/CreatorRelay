import {
  type IconComponent,
  BarChart3,
  BookOpen,
  Bot,
  Globe2,
  LayoutDashboard,
  Library,
  Send,
  Settings2,
} from "../components/Icons";

export type RouteId =
  | "dashboard"
  | "studio"
  | "publishing"
  | "content"
  | "analytics"
  | "assistant"
  | "platforms"
  | "settings";

export type NavGroup = "primary" | "secondary";
export type RouteAvailability = "available" | "development" | "planned";

export interface RouteMeta {
  id: RouteId;
  path: string;
  label: string;
  enLabel: string;
  eyebrow: string;
  description: string;
  priority: "P0" | "P0/P1" | "P1";
  group: NavGroup;
  icon: IconComponent;
  availability: RouteAvailability;
  milestone: string;
  badge?: string;
}

export const PRD_ROUTES: RouteMeta[] = [
  {
    id: "dashboard",
    path: "/dashboard",
    label: "运营总览",
    enLabel: "Dashboard",
    eyebrow: "DASHBOARD",
    description: "全平台运营总览、待办事项、账号健康与异常提醒",
    priority: "P0",
    group: "primary",
    icon: LayoutDashboard,
    availability: "available",
    milestone: "M6",
  },
  {
    id: "studio",
    path: "/studio",
    label: "写作空间",
    enLabel: "Studio",
    eyebrow: "STUDIO",
    description: "视频、图文、图文文章与纯文章创作，AI 辅助和多平台适配稿",
    priority: "P0",
    group: "primary",
    icon: BookOpen,
    availability: "available",
    milestone: "M2 / M5",
  },
  {
    id: "publishing",
    path: "/publishing",
    label: "发布中心",
    enLabel: "Publishing",
    eyebrow: "PUBLISHING",
    description: "多平台发布任务、定时计划、人工辅助与发布结果追踪",
    priority: "P0/P1",
    group: "primary",
    icon: Send,
    availability: "available",
    milestone: "M6",
  },
  {
    id: "content",
    path: "/content",
    label: "内容中心",
    enLabel: "Content",
    eyebrow: "CONTENT",
    description: "跨平台已发布作品聚合、多维筛选与历史表现检索",
    priority: "P0",
    group: "primary",
    icon: Library,
    availability: "available",
    milestone: "M6",
  },
  {
    id: "analytics",
    path: "/analytics",
    label: "数据分析",
    enLabel: "Analytics",
    eyebrow: "ANALYTICS",
    description: "账号粉丝趋势、作品表现分析与跨平台口径对比",
    priority: "P0",
    group: "primary",
    icon: BarChart3,
    availability: "available",
    milestone: "M6",
  },
  {
    id: "assistant",
    path: "/assistant",
    label: "AI 助手",
    enLabel: "Assistant",
    eyebrow: "ASSISTANT",
    description: "BYOK 模型对话、内部工具与平台 MCP 工作流执行",
    priority: "P1",
    group: "secondary",
    icon: Bot,
    availability: "planned",
    milestone: "M7",
  },
  {
    id: "platforms",
    path: "/platforms",
    label: "平台管理",
    enLabel: "Platforms",
    eyebrow: "PLATFORMS",
    description: "平台适配器、多账号绑定、会话隔离与能力状态检测",
    priority: "P0",
    group: "secondary",
    icon: Globe2,
    availability: "available",
    milestone: "M3 / M4",
  },
  {
    id: "settings",
    path: "/settings",
    label: "系统设置",
    enLabel: "Settings",
    eyebrow: "SETTINGS",
    description: "工作区存储、BYOK 模型、MCP Server、插件权限与安全备份",
    priority: "P0",
    group: "secondary",
    icon: Settings2,
    availability: "available",
    milestone: "M1",
  },
];

export const PRIMARY_ROUTES = PRD_ROUTES.filter((route) => route.group === "primary");
export const SECONDARY_ROUTES = PRD_ROUTES.filter((route) => route.group === "secondary");
export const PLANNED_ROUTES = PRD_ROUTES.filter((route) => route.availability === "planned");

export function routeAvailabilityLabel(route: RouteMeta) {
  if (route.availability === "available") return "可用";
  if (route.availability === "development") return "开发中";
  return "规划中";
}

export function findRouteByPathname(pathname: string): RouteMeta {
  return (
    PRD_ROUTES.find((route) => pathname === route.path || pathname.startsWith(`${route.path}/`)) ??
    PRD_ROUTES[1]
  );
}
