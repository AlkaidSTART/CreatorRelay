import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import {
  AlignLeft,
  ArrowDownToLine,
  ArrowLeft,
  ArrowRight,
  ArrowUpRight,
  BookOpen,
  Check,
  ChevronDown,
  ChevronLeft,
  ChevronRight,
  ChevronsUpDown,
  CircleHelp,
  Clock3,
  Command,
  FilePlus2,
  FileText,
  FolderOpen,
  History,
  Image,
  LayoutGrid,
  ListFilter,
  MoreHorizontal,
  PanelLeftClose,
  PanelRightClose,
  Plus,
  Search,
  Settings2,
  Sparkles,
  Tags,
  Upload,
  X,
} from "lucide-react";
import "./App.css";

type Document = {
  id: string;
  title: string;
  path: string;
  updatedAt: string;
  preview: string;
  wordCount: number;
  charCount: number;
  tags: string[];
};

type Revision = { id: number; createdAt: string; label: string; preview: string };
type WorkspaceInfo = { path: string; name: string };
type SaveState = "saved" | "saving" | "unsaved" | "error";

const sampleContent = `---
id: article-wasm-edge
title: 在边缘运行 WebAssembly：一次轻量化实践
created_at: 2026-11-24T10:30:00+08:00
updated_at: 2026-11-25T09:41:00+08:00
tags:
  - WebAssembly
  - 边缘计算
status: draft
---

# 在边缘运行 WebAssembly：一次轻量化实践

当服务离用户更近，延迟自然会低一些。但真正的挑战并不是把容器搬到边缘，而是如何在有限的资源里，运行足够**安全、轻量且可预测**的代码。

最近我把一个数据转换任务迁移到了 WebAssembly 运行时。它并没有让系统架构变得更复杂，反而让部署边界清晰了不少。

## 为什么选择 WebAssembly

对于运行用户提交或团队共享的扩展逻辑，传统容器通常不是最合适的隔离单位。WebAssembly 提供了更细颗粒度的执行边界：

- 启动开销小，可以按请求创建实例
- 默认不具备宿主机访问能力
- 使用 WASI 显式开放文件和网络权限
- 同一份模块可以运行在不同语言的宿主中

```rust
let engine = Engine::default();
let module = Module::from_file(&engine, "transform.wasm")?;
let mut store = Store::new(&engine, context);
let instance = linker.instantiate(&mut store, &module)?;
```

这段代码只是最小入口。生产环境中，资源限制、超时、日志采样和能力白名单同样重要。

## 设计时的几个取舍

我们先把输入输出定义成稳定的 JSON Schema，再把运行时当成一个可替换的执行器。这样可以让本地调试、单元测试和边缘部署保持一致。

> 沙箱边界应该由宿主明确定义，而不是依赖插件作者自觉。

另一个值得注意的细节是**可观测性**。每次调用至少要记录模块摘要、执行时长和资源消耗，而不是把用户数据完整写入日志。

## 最后

这次改造没有追求“把所有东西都放进 Wasm”，而是只迁移了一个输入输出边界明确的任务。对于大多数项目来说，从这个小切口开始，往往比一次性重写更稳妥。
`;

const demoWorkspace: WorkspaceInfo = { path: "~/Documents/CreatorRelay", name: "CreatorRelay" };
const demoDocuments: Document[] = [
  {
    id: "article-wasm-edge",
    title: "在边缘运行 WebAssembly：一次轻量化实践",
    path: "articles/wasm-edge/article.md",
    updatedAt: new Date(Date.now() - 38 * 60 * 1000).toISOString(),
    preview: "当服务离用户更近，延迟自然会低一些。但真正的挑战并不是把容器搬到边缘……",
    wordCount: 684,
    charCount: 1290,
    tags: ["WebAssembly", "边缘计算"],
  },
  {
    id: "article-rust-cli",
    title: "用 Rust 构建一个可靠的命令行工具",
    path: "articles/rust-cli/article.md",
    updatedAt: new Date(Date.now() - 24 * 60 * 60 * 1000).toISOString(),
    preview: "一个好用的 CLI，不只是参数解析。它还需要在错误发生时给出清晰的下一步……",
    wordCount: 1024,
    charCount: 2018,
    tags: ["Rust", "工程实践"],
  },
  {
    id: "article-design-tokens",
    title: "设计 Token：从视觉约束到工程语言",
    path: "articles/design-tokens/article.md",
    updatedAt: new Date(Date.now() - 3 * 24 * 60 * 60 * 1000).toISOString(),
    preview: "设计 Token 不应该只是颜色变量。它更像是设计决策和代码实现之间的一份契约……",
    wordCount: 832,
    charCount: 1680,
    tags: ["设计系统", "前端"],
  },
  {
    id: "article-observability",
    title: "小团队如何建立有效的可观测性",
    path: "articles/observability/article.md",
    updatedAt: new Date(Date.now() - 7 * 24 * 60 * 60 * 1000).toISOString(),
    preview: "监控系统不应该只在故障之后才被想起。真正有效的信号，往往来自很小的约定……",
    wordCount: 562,
    charCount: 1054,
    tags: ["可观测性", "团队效率"],
  },
];

const starterRevisions: Revision[] = [
  { id: 3, createdAt: "今天 09:41", label: "当前版本", preview: "当服务离用户更近，延迟自然会低一些。但真正的挑战并不是把容器搬到边缘……" },
  { id: 2, createdAt: "今天 09:18", label: "自动保存", preview: "最近我把一个数据转换任务迁移到了 WebAssembly 运行时。它并没有让系统架构……" },
  { id: 1, createdAt: "昨天 16:32", label: "初始版本", preview: "当服务离用户更近，延迟自然会低一些。真正的挑战是如何在有限资源里运行代码。" },
];

const navItems = [
  { id: "writing", label: "写作空间", icon: BookOpen },
  { id: "distribution", label: "分发准备", icon: ArrowUpRight, soon: true },
];

function timeLabel(value: string) {
  if (value.includes("今天") || value.includes("昨天")) return value;
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "刚刚";
  const diff = Date.now() - date.getTime();
  if (diff < 60 * 60 * 1000) return `${Math.max(1, Math.floor(diff / 60000))} 分钟前`;
  if (diff < 24 * 60 * 60 * 1000) return `${Math.floor(diff / 3600000)} 小时前`;
  if (diff < 2 * 24 * 60 * 60 * 1000) return "昨天";
  return new Intl.DateTimeFormat("zh-CN", { month: "short", day: "numeric" }).format(date);
}

function bodyFromMarkdown(markdown: string) {
  if (markdown.startsWith("---\n")) {
    const end = markdown.indexOf("\n---", 4);
    if (end >= 0) return markdown.slice(end + 5).replace(/^\n/, "");
  }
  return markdown;
}

function titleFromMarkdown(markdown: string, fallback: string) {
  const frontmatter = markdown.match(/^---\n([\s\S]*?)\n---/);
  const title = frontmatter?.[1].match(/^title:\s*["']?(.*?)["']?\s*$/m)?.[1];
  if (title) return title;
  return markdown.match(/^#\s+(.+)$/m)?.[1]?.trim() ?? fallback;
}

function countWords(markdown: string) {
  return bodyFromMarkdown(markdown).trim().split(/\s+/u).filter(Boolean).length;
}

function getStoredDocuments(): Document[] | null {
  try {
    const parsed = JSON.parse(localStorage.getItem("creatorrelay.demo.documents") ?? "null");
    return Array.isArray(parsed) ? parsed : null;
  } catch {
    return null;
  }
}

function App() {
  const [workspace, setWorkspace] = useState<WorkspaceInfo | null>(null);
  const [documents, setDocuments] = useState<Document[]>([]);
  const [selectedId, setSelectedId] = useState("");
  const [content, setContent] = useState("");
  const [baseHash, setBaseHash] = useState("");
  const [query, setQuery] = useState("");
  const [saveState, setSaveState] = useState<SaveState>("saved");
  const [viewMode, setViewMode] = useState<"write" | "preview">("write");
  const [showHistory, setShowHistory] = useState(false);
  const [revisions, setRevisions] = useState<Revision[]>(starterRevisions);
  const [isCreating, setIsCreating] = useState(false);
  const [newTitle, setNewTitle] = useState("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [sidebarOpen, setSidebarOpen] = useState(true);
  const [showDemoBanner, setShowDemoBanner] = useState(false);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const selected = documents.find((document) => document.id === selectedId) ?? null;
  const body = useMemo(() => bodyFromMarkdown(content), [content]);
  const words = useMemo(() => countWords(content), [content]);
  const chars = useMemo(() => body.replace(/\s/gu, "").length, [body]);

  const loadDocuments = useCallback(async (filter = "") => {
    if (isTauri()) {
      const items = await invoke<Document[]>("list_documents", { query: filter || null });
      setDocuments(items);
      if (items.length && !items.some((item) => item.id === selectedId)) setSelectedId(items[0].id);
      return items;
    }
    const stored = getStoredDocuments();
    const items = stored ?? demoDocuments;
    const term = filter.trim().toLocaleLowerCase();
    const filtered = term
      ? items.filter((item) => `${item.title} ${item.preview} ${item.tags.join(" ")}`.toLocaleLowerCase().includes(term))
      : items;
    setDocuments(filtered);
    if (filtered.length && !filtered.some((item) => item.id === selectedId)) setSelectedId(filtered[0].id);
    return filtered;
  }, [selectedId]);

  const loadDocument = useCallback(async (document: Document) => {
    setError("");
    try {
      let markdown: string;
      if (isTauri()) {
        markdown = await invoke<string>("read_document", { path: document.path });
      } else {
        markdown = localStorage.getItem(`creatorrelay.demo.content.${document.id}`) ?? (document.id === demoDocuments[0].id ? sampleContent : `---\nid: ${document.id}\ntitle: "${document.title}"\ncreated_at: 2026-11-24T10:00:00+08:00\nupdated_at: 2026-11-25T09:00:00+08:00\ntags: []\nstatus: draft\n---\n\n# ${document.title}\n\n${document.preview.replace("……", "。")}`);
      }
      setContent(markdown);
      setBaseHash(await digest(markdown));
      setSaveState("saved");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
      setSaveState("error");
    }
  }, []);

  useEffect(() => {
    let cancelled = false;
    async function initialize() {
      try {
        if (isTauri()) {
          const active = await invoke<WorkspaceInfo | null>("get_workspace");
          if (cancelled) return;
          setWorkspace(active);
          if (active) {
            const items = await invoke<Document[]>("list_documents", { query: null });
            if (!cancelled) {
              setDocuments(items);
              if (items.length) setSelectedId(items[0].id);
            }
          }
        } else {
          setWorkspace(demoWorkspace);
          setShowDemoBanner(true);
          const items = getStoredDocuments() ?? demoDocuments;
          setDocuments(items);
          if (items.length) setSelectedId(items[0].id);
        }
      } catch (cause) {
        if (!cancelled) setError(cause instanceof Error ? cause.message : String(cause));
      } finally {
        if (!cancelled) setLoading(false);
      }
    }
    void initialize();
    return () => { cancelled = true; };
  }, []);

  useEffect(() => {
    if (!selected) return;
    void loadDocument(selected);
  }, [selectedId]);

  useEffect(() => {
    if (!selected || saveState !== "unsaved") return;
    const timer = window.setTimeout(async () => {
      setSaveState("saving");
      try {
        if (isTauri()) {
          await invoke<Document>("save_document", { path: selected.path, content, baseHash });
        } else {
          localStorage.setItem(`creatorrelay.demo.content.${selected.id}`, content);
          const updated = { ...selected, title: titleFromMarkdown(content, selected.title), preview: bodyFromMarkdown(content).split("\n").find((line) => line.trim() && !line.startsWith("#"))?.slice(0, 120) ?? "", updatedAt: new Date().toISOString(), wordCount: words, charCount: chars };
          const all = getStoredDocuments() ?? demoDocuments;
          const next = all.map((document) => document.id === selected.id ? updated : document);
          localStorage.setItem("creatorrelay.demo.documents", JSON.stringify(next));
          setDocuments((current) => current.map((document) => document.id === selected.id ? updated : document));
        }
        setBaseHash(await digest(content));
        setSaveState("saved");
        setNotice("已保存到工作区");
        window.setTimeout(() => setNotice(""), 1800);
      } catch (cause) {
        setSaveState("error");
        setError(cause instanceof Error ? cause.message : String(cause));
      }
    }, 650);
    return () => window.clearTimeout(timer);
  }, [content, saveState, selectedId]);

  useEffect(() => {
    if (!selected || isTauri()) return;
    const saved = localStorage.getItem(`creatorrelay.demo.content.${selected.id}`);
    if (!saved) localStorage.setItem(`creatorrelay.demo.content.${selected.id}`, selected.id === demoDocuments[0].id ? sampleContent : `---\nid: ${selected.id}\ntitle: "${selected.title}"\ncreated_at: 2026-11-24T10:00:00+08:00\nupdated_at: 2026-11-25T09:00:00+08:00\ntags: []\nstatus: draft\n---\n\n# ${selected.title}\n\n${selected.preview.replace("……", "。")}`);
  }, [selectedId]);

  useEffect(() => {
    if (!selected || !isTauri()) return;
    invoke<Revision[]>("list_revisions", { path: selected.path }).then(setRevisions).catch(() => setRevisions([]));
  }, [selectedId, saveState]);

  async function digest(value: string) {
    if (!crypto.subtle) return "";
    const bytes = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(value));
    return Array.from(new Uint8Array(bytes)).map((byte) => byte.toString(16).padStart(2, "0")).join("");
  }

  async function chooseWorkspace() {
    setError("");
    try {
      if (!isTauri()) {
        setNotice("请在桌面应用中选择真实工作区");
        window.setTimeout(() => setNotice(""), 2200);
        return;
      }
      const path = await open({ directory: true, multiple: false, title: "选择 CreatorRelay 工作区" });
      if (typeof path !== "string") return;
      const next = await invoke<WorkspaceInfo>("set_workspace_path", { path });
      setWorkspace(next);
      const items = await invoke<Document[]>("list_documents", { query: null });
      setDocuments(items);
      if (items.length) setSelectedId(items[0].id);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  }

  async function createDocument() {
    const title = newTitle.trim();
    if (!title) return;
    try {
      let created: Document;
      if (isTauri()) {
        created = await invoke<Document>("create_document", { title });
      } else {
        const id = `article-${crypto.randomUUID()}`;
        created = { id, title, path: `articles/${id}/article.md`, updatedAt: new Date().toISOString(), preview: "开始写下你的想法……", wordCount: 0, charCount: 0, tags: [] };
        const all = getStoredDocuments() ?? demoDocuments;
        localStorage.setItem("creatorrelay.demo.documents", JSON.stringify([created, ...all]));
        localStorage.setItem(`creatorrelay.demo.content.${id}`, `---\nid: ${id}\ntitle: "${title}"\ncreated_at: ${new Date().toISOString()}\nupdated_at: ${new Date().toISOString()}\ntags: []\nstatus: draft\n---\n\n# ${title}\n\n`);
      }
      setNewTitle("");
      setIsCreating(false);
      await loadDocuments();
      setSelectedId(created.id);
      setViewMode("write");
      window.setTimeout(() => textareaRef.current?.focus(), 60);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  }

  async function restoreRevision(revision: Revision) {
    if (!selected) return;
    try {
      if (isTauri()) {
        const updated = await invoke<Document>("restore_revision", { path: selected.path, revisionId: revision.id });
        await loadDocuments();
        await loadDocument(updated);
      } else {
        setNotice("演示模式下仅展示版本记录，桌面端可恢复完整历史版本");
        window.setTimeout(() => setNotice(""), 2500);
      }
      setShowHistory(false);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  }

  function updateContent(value: string) {
    setContent(value);
    setSaveState("unsaved");
  }

  const saveLabel = saveState === "saving" ? "正在保存" : saveState === "unsaved" ? "未保存" : saveState === "error" ? "保存失败" : "所有更改已保存";
  const platformCount = 2;

  if (loading) return <div className="app-loading"><span className="loading-mark">cr</span><span>正在整理你的工作区</span></div>;

  return (
    <div className="app-shell">
      <aside className={`sidebar ${sidebarOpen ? "" : "sidebar-collapsed"}`}>
        <div className="brand-row">
          <div className="brand-mark">cr</div>
          {sidebarOpen && <><span className="brand-name">creatorrelay</span><button className="icon-button sidebar-toggle" aria-label="收起侧栏" onClick={() => setSidebarOpen(false)}><PanelLeftClose size={16} /></button></>}
        </div>
        {!sidebarOpen && <button className="icon-button sidebar-toggle-collapsed" aria-label="展开侧栏" onClick={() => setSidebarOpen(true)}><PanelLeftClose size={16} /></button>}

        <button className="workspace-select" onClick={chooseWorkspace}>
          <span className="workspace-avatar">{workspace?.name.slice(0, 1).toUpperCase() ?? "C"}</span>
          {sidebarOpen && <><span className="workspace-copy"><strong>{workspace?.name ?? "选择工作区"}</strong><small>个人空间</small></span><ChevronsUpDown size={14} className="muted-icon" /></>}
        </button>

        {sidebarOpen && <>
          <div className="side-section-label">工作区</div>
          <nav className="primary-nav" aria-label="主导航">
            {navItems.map((item) => {
              const Icon = item.icon;
              return <button key={item.id} className={`nav-item ${item.id === "writing" ? "active" : ""}`} onClick={() => item.soon && setNotice("分发准备将在后续版本开放") }><Icon size={17} strokeWidth={1.7} /><span>{item.label}</span>{item.soon && <span className="nav-count">2</span>}</button>;
            })}
          </nav>

          <div className="sidebar-divider" />
          <div className="list-heading"><span className="side-section-label">最近编辑</span><button className="tiny-icon-button" aria-label="筛选"><ListFilter size={14} /></button></div>
          <div className="document-list" role="list">
            {documents.map((document) => <button key={document.id} className={`document-item ${document.id === selectedId ? "selected" : ""}`} onClick={() => setSelectedId(document.id)} role="listitem">
              <FileText size={16} strokeWidth={1.7} className="document-icon" />
              <span className="document-item-copy"><strong>{document.title}</strong><small>{timeLabel(document.updatedAt)}</small></span>
              {document.id === selectedId && <span className="selected-indicator" />}
            </button>)}
            {!documents.length && <div className="empty-docs">没有找到匹配的稿件</div>}
          </div>
          <button className="new-document-side" onClick={() => setIsCreating(true)}><Plus size={15} /><span>新建文章</span><kbd>⌘ N</kbd></button>

          <div className="sidebar-bottom">
            <button className="nav-item muted-nav" onClick={() => setNotice("AI 写作设置将在后续迭代开放") }><Sparkles size={17} strokeWidth={1.7} /><span>AI 写作</span><span className="setting-dot" /></button>
            <button className="nav-item muted-nav" onClick={() => setNotice("设置面板正在准备中") }><Settings2 size={17} strokeWidth={1.7} /><span>设置</span></button>
            <div className="profile-row"><div className="profile-avatar">林</div><span className="profile-copy"><strong>创作者</strong><small>本地工作区</small></span><MoreHorizontal size={17} className="muted-icon" /></div>
          </div>
        </>}
      </aside>

      <main className="main-column">
        {showDemoBanner && <div className="demo-notice"><span><strong>预览模式</strong>　以下为本地演示内容，桌面版将读写你选择的 SQLite 工作区。</span><button onClick={() => setShowDemoBanner(false)} aria-label="关闭提示"><X size={14} /></button></div>}
        <header className="topbar">
          <div className="breadcrumb"><span>写作空间</span><ChevronRight size={14} /><strong>{selected?.title ?? "未选择文章"}</strong></div>
          <div className="topbar-actions"><button className="icon-button help-button" aria-label="帮助" onClick={() => setNotice("CreatorRelay 本地创作工作台") }><CircleHelp size={17} /></button><button className="avatar-small" aria-label="个人资料">林</button></div>
        </header>

        <section className="editor-toolbar">
          <div className="editor-status"><span className={`save-indicator ${saveState}`} />{saveLabel}<span className="status-divider" /> <FolderOpen size={13} />{workspace?.path ?? "未选择工作区"}</div>
          <div className="toolbar-actions"><button className={`toolbar-button ${viewMode === "write" ? "selected" : ""}`} onClick={() => setViewMode("write")}><AlignLeft size={15} />编辑</button><button className={`toolbar-button ${viewMode === "preview" ? "selected" : ""}`} onClick={() => setViewMode("preview")}><BookOpen size={15} />预览</button><span className="toolbar-divider" /><button className="icon-button" onClick={() => setShowHistory((value) => !value)} aria-label="版本历史"><History size={16} /></button><button className="icon-button" onClick={() => setNotice("导出功能将在下一迭代开放") } aria-label="导出"><ArrowDownToLine size={16} /></button><button className="publish-button" onClick={() => setNotice("人工发布准备页将在本迭代后续接入")}>发布准备<ArrowRight size={14} /></button></div>
        </section>

        {error && <div className="error-banner"><span>{error}</span><button onClick={() => setError("")} aria-label="关闭错误"><X size={14} /></button></div>}
        {notice && <div className="toast-notice" role="status">{notice}</div>}

        <div className={`editor-workspace ${showHistory ? "with-history" : ""}`}>
          <section className="writing-pane">
            <div className="editor-paper">
              {selected ? <>
                <div className="paper-overline"><span className="overline-rule" />ARTICLE DRAFT <span className="overline-dot">/</span> {selected.path.split("/")[1]?.toUpperCase() ?? "LOCAL"}</div>
                {viewMode === "write" ? <textarea ref={textareaRef} className="markdown-editor" aria-label="Markdown 正文" spellCheck={false} value={content} onChange={(event) => updateContent(event.target.value)} /> : <article className="markdown-preview"><ReactMarkdown remarkPlugins={[remarkGfm]}>{body}</ReactMarkdown></article>}
                {viewMode === "write" && <div className="editor-bottom-hint"><Command size={12} /><span>输入 <kbd>/</kbd> 打开命令菜单</span><span className="hint-separator">·</span><span>Markdown 已启用</span></div>}
              </> : <div className="no-document"><FileText size={28} /><strong>选择一篇文章开始写作</strong><span>或者创建一篇新的 Markdown 草稿</span><button className="primary-button" onClick={() => setIsCreating(true)}><Plus size={15} />新建文章</button></div>}
            </div>
            <footer className="editor-footer"><div className="footer-left"><span><AlignLeft size={13} />Markdown</span><span className="footer-dot" /> <span>{words.toLocaleString()} 个词</span><span className="footer-dot" /><span>{chars.toLocaleString()} 字符</span></div><div className="footer-right"><span className="keyboard-hint"><Command size={11} /> S</span><span>保存</span><span className="footer-dot" /><span>UTF-8</span></div></footer>
          </section>

          <aside className="context-pane">
            <div className="context-header"><div><span className="context-eyebrow">DOCUMENT</span><h2>文章信息</h2></div><button className="icon-button" aria-label="更多文章选项"><MoreHorizontal size={17} /></button></div>
            {selected ? <>
              <div className="context-section"><div className="section-caption">发布平台 <button className="text-action" onClick={() => setNotice("平台适配将在后续迭代开放")}>管理</button></div><div className="platform-row"><div className="platform-mark juejin-mark">稀</div><div className="platform-meta"><strong>掘金</strong><small>平台稿未创建</small></div><span className="platform-status idle" /><button className="tiny-icon-button" aria-label="为掘金创建平台稿" onClick={() => setNotice("平台稿生成将随 AI 写作功能开放")}><Plus size={15} /></button></div><div className="platform-row"><div className="platform-mark csdn-mark">C</div><div className="platform-meta"><strong>CSDN</strong><small>平台稿未创建</small></div><span className="platform-status idle" /><button className="tiny-icon-button" aria-label="为 CSDN 创建平台稿" onClick={() => setNotice("平台稿生成将随 AI 写作功能开放")}><Plus size={15} /></button></div><div className="platform-note"><span className="note-line" />平台稿独立保存，主稿更新不会覆盖你的修改。</div></div>
              <div className="context-section"><div className="section-caption">标签 <button className="tiny-icon-button" aria-label="编辑标签"><Plus size={14} /></button></div><div className="tag-list">{selected.tags.length ? selected.tags.map((tag) => <span className="tag-chip" key={tag}>{tag}</span>) : <button className="add-tag" onClick={() => setNotice("标签编辑正在准备中")}><Tags size={13} />添加标签</button>}</div></div>
              <div className="context-section"><div className="section-caption">素材 <button className="tiny-icon-button" aria-label="导入素材" onClick={() => setNotice("素材导入将在下一步接入工作区文件系统")}><Plus size={14} /></button></div><button className="asset-empty" onClick={() => setNotice("支持拖入图片素材，素材目录接口正在开发中")}><Image size={16} /><span>将图片拖到编辑区，或从本地添加</span><ChevronRight size={14} /></button></div>
              <div className="context-section source-version"><div className="section-caption">来源版本</div><div className="source-card"><div className="source-icon"><Clock3 size={14} /></div><div><strong>主稿版本 03</strong><small>{timeLabel(selected.updatedAt)} · 当前版本</small></div><Check size={15} className="source-check" /></div></div>
              <div className="context-spacer" />
              <div className="context-bottom"><div className="reading-time"><span>预计阅读</span><strong>{Math.max(1, Math.ceil(words / 300))} 分钟</strong></div><button className="context-more" onClick={() => setShowHistory(true)}><History size={14} />查看版本历史<ChevronRight size={14} /></button></div>
            </> : <div className="context-empty">选择文章后查看详情</div>}
          </aside>

          {showHistory && <aside className="history-drawer"><div className="history-heading"><div><span className="context-eyebrow">HISTORY</span><h2>版本历史</h2></div><button className="icon-button" onClick={() => setShowHistory(false)} aria-label="关闭版本历史"><X size={16} /></button></div><p className="history-description">编辑过程会自动保留快照。恢复版本将创建新的当前版本。</p><div className="history-list">{revisions.map((revision, index) => <div className={`revision-item ${index === 0 ? "current" : ""}`} key={revision.id}><div className="revision-marker"><span /></div><div className="revision-content"><div className="revision-title"><strong>{revision.label}</strong>{index === 0 && <span className="current-pill">当前</span>}</div><small>{timeLabel(revision.createdAt)}</small><p>{revision.preview}</p>{index > 0 && <button className="restore-button" onClick={() => restoreRevision(revision)}>恢复此版本<ArrowRight size={13} /></button>}</div></div>)}</div><div className="history-footnote"><History size={14} /><span>最多保留最近 30 个版本</span></div></aside>}
        </div>
      </main>

      <aside className={`document-rail ${sidebarOpen ? "" : "rail-wide"}`}>
        <div className="rail-search"><Search size={15} /><input value={query} onChange={(event) => { setQuery(event.target.value); void loadDocuments(event.target.value); }} placeholder="搜索文章" aria-label="搜索文章" /><kbd>⌘ K</kbd></div>
        <div className="rail-title-row"><div><span className="context-eyebrow">YOUR LIBRARY</span><h1>文章 <span>{documents.length.toString().padStart(2, "0")}</span></h1></div><button className="create-icon-button" aria-label="新建文章" onClick={() => setIsCreating(true)}><FilePlus2 size={17} /></button></div>
        <div className="rail-filter-row"><span>最近编辑</span><button className="filter-button" onClick={() => setNotice("已按最近编辑排序")}><ChevronDown size={13} /></button></div>
        <div className="rail-document-list">{documents.map((document, index) => <button key={document.id} className={`rail-document ${document.id === selectedId ? "active" : ""}`} onClick={() => setSelectedId(document.id)}><div className="rail-doc-top"><span className="rail-doc-date">{timeLabel(document.updatedAt)}</span><span className={`rail-doc-index ${document.id === selectedId ? "current" : ""}`}>{String(index + 1).padStart(2, "0")}</span></div><strong>{document.title}</strong><p>{document.preview}</p><div className="rail-doc-tags">{document.tags.slice(0, 2).map((tag) => <span key={tag}>{tag}</span>)}</div></button>)}{!documents.length && <div className="rail-empty">暂无文章</div>}</div>
        <div className="rail-bottom"><div className="storage-card"><div className="storage-card-icon"><FolderOpen size={15} /></div><div className="storage-card-copy"><strong>本地工作区</strong><span>{workspace?.path ?? "尚未设置路径"}</span></div><button className="tiny-icon-button" onClick={chooseWorkspace} aria-label="更换工作区"><ArrowUpRight size={14} /></button></div><div className="privacy-caption"><span className="privacy-dot" />你的文章保存在本地设备</div></div>
      </aside>

      {isCreating && <div className="modal-backdrop" onMouseDown={(event) => { if (event.target === event.currentTarget) setIsCreating(false); }}><form className="create-dialog" onSubmit={(event) => { event.preventDefault(); void createDocument(); }}><div className="dialog-icon"><FilePlus2 size={19} /></div><button type="button" className="icon-button dialog-close" onClick={() => setIsCreating(false)} aria-label="关闭"><X size={16} /></button><span className="context-eyebrow">NEW DOCUMENT</span><h2>开始一篇新文章</h2><p>创建一个独立的 Markdown 主稿，内容保存在当前工作区。</p><label htmlFor="document-title">文章标题</label><input id="document-title" autoFocus value={newTitle} onChange={(event) => setNewTitle(event.target.value)} placeholder="例如：一次关于边缘计算的实践" /><div className="dialog-actions"><button type="button" className="secondary-button" onClick={() => setIsCreating(false)}>取消</button><button type="submit" className="primary-button" disabled={!newTitle.trim()}><Plus size={15} />创建文章</button></div></form></div>}
    </div>
  );
}

export default App;
