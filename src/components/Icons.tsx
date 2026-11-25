import type { CSSProperties, JSX } from "react";

export type IconProps = {
  className?: string;
  size?: number | string;
  strokeWidth?: number;
  style?: CSSProperties;
};

export function createIcon(glyph: string) {
  return function AppIcon({ className = "", size, style }: IconProps) {
    return (
      <i
        className={`fa-solid fa-${glyph} ${className}`.trim()}
        style={{ fontSize: size, lineHeight: 1, ...style }}
        aria-hidden="true"
      />
    );
  };
}

export const AlertCircle = createIcon("circle-exclamation");
export const AlertTriangle = createIcon("triangle-exclamation");
export const AlignLeft = createIcon("align-left");
export const ArrowDownToLine = createIcon("download");
export const ArrowRight = createIcon("arrow-right");
export const ArrowUpRight = createIcon("arrow-up-right-from-square");
export const BarChart3 = createIcon("chart-column");
export const BookOpen = createIcon("book-open");
export const Bot = createIcon("robot");
export const Check = createIcon("check");
export const CheckCircle2 = createIcon("circle-check");
export const ChevronRight = createIcon("chevron-right");
export const ChevronsUpDown = createIcon("sort");
export const Clock3 = createIcon("clock");
export const Command = createIcon("keyboard");
export const ExternalLink = createIcon("arrow-up-right-from-square");
export const FilePlus2 = createIcon("file-circle-plus");
export const FileText = createIcon("file-lines");
export const FolderOpen = createIcon("folder-open");
export const Globe2 = createIcon("globe");
export const History = createIcon("clock-rotate-left");
export const Image = createIcon("image");
export const Info = createIcon("circle-info");
export const KeyRound = createIcon("key");
export const LayoutDashboard = createIcon("gauge-high");
export const Library = createIcon("layer-group");
export const Palette = createIcon("palette");
export const PanelLeft = createIcon("table-columns");
export const PanelLeftClose = createIcon("angles-left");
export const PanelLeftOpen = createIcon("angles-right");
export const Plug = createIcon("plug");
export const Plus = createIcon("plus");
export const Pencil = createIcon("pencil");
export const Play = createIcon("play");
export const RefreshCw = createIcon("arrows-rotate");
export const Search = createIcon("magnifying-glass");
export const Send = createIcon("paper-plane");
export const Server = createIcon("server");
export const Settings2 = createIcon("gear");
export const Shield = createIcon("shield");
export const ShieldAlert = createIcon("shield-halved");
export const ShieldCheck = createIcon("shield");
export const Sparkles = createIcon("wand-magic-sparkles");
export const Tags = createIcon("tags");
export const Terminal = createIcon("terminal");
export const Trash2 = createIcon("trash-can");
export const TrendingUp = createIcon("chart-line");
export const X = createIcon("xmark");

export type IconComponent = (props: IconProps) => JSX.Element;
export const ArrowDown = createIcon("arrow-down");
export const ArrowUp = createIcon("arrow-up");
export const Eye = createIcon("eye");
export const LoaderCircle = createIcon("spinner");
export const RotateCcw = createIcon("rotate-left");
export const Star = createIcon("star");
export const Video = createIcon("video");
export const Copy = createIcon("copy");
export const GripVertical = createIcon("grip-vertical");
export const Save = createIcon("floppy-disk");

export const Link2 = createIcon("link");
