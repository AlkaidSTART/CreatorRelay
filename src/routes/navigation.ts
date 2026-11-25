import {
  type IconComponent,
  BarChart3,
  BookOpen,
  Bot,
  Globe2,
  LayoutDashboard,
  Library,
  Link2,
  Plug,
  Send,
  Settings2,
  ShieldCheck,
  Sparkles,
  Terminal,
} from "../components/Icons";

export type RouteId =
  | "dashboard"
  | "studio"
  | "publishing"
  | "content"
  | "analytics"
  | "assistant"
  | "platforms"
  | "adapters"
  | "creator-centers"
  | "ecosystem"
  | "settings"
  | "settings-ai"
  | "settings-diagnostics";

export type NavGroup = "workspace" | "ecosystem" | "system";
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
  exact?: boolean;
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
    group: "workspace",
    icon: LayoutDashboard,
    availability: "available",
    milestone: "M6",
  },
  {
    id: "studio",
    path: "/studio",
    label: "内容工作室",
    enLabel: "Studio",
    eyebrow: "STUDIO",
    description: "视频、图文、图文文章与纯文章创作，AI 辅助和多平台适配稿",
    priority: "P0",
    group: "workspace",
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
    group: "workspace",
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
    group: "workspace",
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
    group: "workspace",
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
    group: "workspace",
    icon: Bot,
    availability: "available",
    milestone: "M7",
  },
  {
    id: "platforms",
    path: "/platforms",
    label: "平台账号",
    enLabel: "Platforms",
    eyebrow: "PLATFORMS",
    description: "多账号绑定、真实会话校验、系统凭据与会话容器隔离",
    priority: "P0",
    group: "ecosystem",
    icon: Globe2,
    availability: "available",
    milestone: "M3",
  },
  {
    id: "adapters",
    path: "/adapters",
    label: "适配器与插件",
    enLabel: "Adapters",
    eyebrow: "ADAPTERS",
    description: "发现、安装并审查平台适配器，按内容形态探测真实能力",
    priority: "P0",
    group: "ecosystem",
    icon: Plug,
    availability: "available",
    milestone: "M4",
  },
  {
    id: "creator-centers",
    path: "/creator-centers",
    label: "创作者中心",
    enLabel: "Creator Centers",
    eyebrow: "CREATOR CENTERS",
    description: "管理平台创作者中心网址并查看 MCP 能力边界",
    priority: "P1",
    group: "ecosystem",
    icon: Link2,
    availability: "available",
    milestone: "M7",
  },
  {
    id: "ecosystem",
    path: "/ecosystem",
    label: "生态治理",
    enLabel: "Ecosystem",
    eyebrow: "ECOSYSTEM",
    description: "核对平台准入来源、账号级逐形态能力矩阵与插件市场门禁",
    priority: "P1",
    group: "ecosystem",
    icon: ShieldCheck,
    availability: "available",
    milestone: "M8",
  },
  {
    id: "settings",
    path: "/settings",
    label: "常规与外观",
    enLabel: "Settings",
    eyebrow: "SETTINGS",
    description: "外观主题、本地工作区路径与索引安全备份",
    priority: "P0",
    group: "system",
    icon: Settings2,
    availability: "available",
    milestone: "M1",
    exact: true,
  },
  {
    id: "settings-ai",
    path: "/settings/ai",
    label: "AI 模型配置",
    enLabel: "AI Models",
    eyebrow: "AI MODELS",
    description: "配置 BYOK 供应商、兼容端点、本地模型与系统钥匙串密钥",
    priority: "P0",
    group: "system",
    icon: Sparkles,
    availability: "available",
    milestone: "M5",
  },
  {
    id: "settings-diagnostics",
    path: "/settings/diagnostics",
    label: "运行时诊断",
    enLabel: "Diagnostics",
    eyebrow: "DIAGNOSTICS",
    description: "查看数据库 schema、脱敏日志路径与 IPC/数据库诊断探针",
    priority: "P1",
    group: "system",
    icon: Terminal,
    availability: "available",
    milestone: "M1",
  },
];

export const WORKSPACE_ROUTES = PRD_ROUTES.filter((route) => route.group === "workspace");
export const ECOSYSTEM_ROUTES = PRD_ROUTES.filter((route) => route.group === "ecosystem");
export const SYSTEM_ROUTES = PRD_ROUTES.filter((route) => route.group === "system");

export const NAV_GROUPS: Array<{ id: NavGroup; label: string; routes: RouteMeta[] }> = [
  { id: "workspace", label: "创作运营", routes: WORKSPACE_ROUTES },
  { id: "ecosystem", label: "平台与扩展", routes: ECOSYSTEM_ROUTES },
  { id: "system", label: "系统", routes: SYSTEM_ROUTES },
];

export const PRIMARY_ROUTES = WORKSPACE_ROUTES;
export const SECONDARY_ROUTES = [...ECOSYSTEM_ROUTES, ...SYSTEM_ROUTES];
export const PLANNED_ROUTES = PRD_ROUTES.filter((route) => route.availability === "planned");

const ROUTES_BY_SPECIFICITY = [...PRD_ROUTES].sort((a, b) => b.path.length - a.path.length);

export function routeAvailabilityLabel(route: RouteMeta) {
  if (route.availability === "available") return "可用";
  if (route.availability === "development") return "开发中";
  return "规划中";
}

export function findRouteByPathname(pathname: string): RouteMeta {
  return (
    ROUTES_BY_SPECIFICITY.find(
      (route) => pathname === route.path || pathname.startsWith(`${route.path}/`),
    ) ?? PRD_ROUTES[1]
  );
}
