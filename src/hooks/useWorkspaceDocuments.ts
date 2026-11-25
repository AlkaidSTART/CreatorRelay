import { useCallback, useEffect, useMemo, useRef } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { demoDocuments, demoWorkspace, sampleContent } from "../data/demoData";
import { useAppStore, type Document, type Revision, type WorkspaceInfo } from "../stores/appStore";
import { bodyFromMarkdown, countWords, titleFromMarkdown } from "../utils/markdown";

async function digest(value: string) {
  if (!crypto.subtle) return "";
  const bytes = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(value));
  return Array.from(new Uint8Array(bytes))
    .map((byte) => byte.toString(16).padStart(2, "0"))
    .join("");
}

function getStoredDocuments(): Document[] | null {
  try {
    const parsed = JSON.parse(localStorage.getItem("creatorrelay.demo.documents") ?? "null");
    return Array.isArray(parsed) ? parsed : null;
  } catch {
    return null;
  }
}

export function formatRelativeTime(value: string) {
  if (value.includes("今天") || value.includes("昨天")) return value;
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "刚刚";
  const diff = Date.now() - date.getTime();
  if (diff < 60 * 60 * 1000) return `${Math.max(1, Math.floor(diff / 60000))} 分钟前`;
  if (diff < 24 * 60 * 60 * 1000) return `${Math.floor(diff / 3600000)} 小时前`;
  if (diff < 2 * 24 * 60 * 60 * 1000) return "昨天";
  return new Intl.DateTimeFormat("zh-CN", { month: "short", day: "numeric" }).format(date);
}

export function useWorkspaceDocuments() {
  const {
    workspace,
    setWorkspace,
    documents,
    setDocuments,
    selectedId,
    setSelectedId,
    content,
    setContent,
    updateContent,
    baseHash,
    setBaseHash,
    query,
    setQuery,
    saveState,
    setSaveState,
    viewMode,
    setViewMode,
    showHistory,
    setShowHistory,
    revisions,
    setRevisions,
    isCreating,
    setIsCreating,
    newTitle,
    setNewTitle,
    loading,
    setLoading,
    error,
    setError,
    notice,
    setNotice,
    sidebarOpen,
    setSidebarOpen,
    showDemoBanner,
    setShowDemoBanner,
  } = useAppStore();

  const contentRef = useRef(content);
  const saveRequestRef = useRef(0);
  const saveInFlightRef = useRef(false);
  const documentLoadRef = useRef(0);
  contentRef.current = content;

  const selected = useMemo(
    () => documents.find((document) => document.id === selectedId) ?? null,
    [documents, selectedId],
  );
  const body = useMemo(() => bodyFromMarkdown(content), [content]);
  const words = useMemo(() => countWords(content), [content]);
  const chars = useMemo(() => body.replace(/\s/gu, "").length, [body]);

  const showToast = useCallback(
    (message: string, durationMs = 2000) => {
      setNotice(message);
      window.setTimeout(() => setNotice(""), durationMs);
    },
    [setNotice],
  );

  const loadDocuments = useCallback(
    async (filter = "") => {
      if (isTauri()) {
        const items = await invoke<Document[]>("list_documents", { query: filter || null });
        setDocuments(items);
        return items;
      }
      const stored = getStoredDocuments();
      const items = stored ?? demoDocuments;
      const term = filter.trim().toLocaleLowerCase();
      const filtered = term
        ? items.filter((item) =>
            `${item.title} ${item.preview} ${item.tags.join(" ")}`
              .toLocaleLowerCase()
              .includes(term),
          )
        : items;
      setDocuments(filtered);
      return filtered;
    },
    [setDocuments],
  );

  const loadDocument = useCallback(
    async (document: Document) => {
      const loadId = ++documentLoadRef.current;
      setError("");
      try {
        let markdown: string;
        let contentHash = "";
        if (isTauri()) {
          const result = await invoke<{ content: string; contentHash: string }>("read_document", {
            path: document.path,
          });
          markdown = result.content;
          contentHash = result.contentHash;
        } else {
          markdown =
            localStorage.getItem(`creatorrelay.demo.content.${document.id}`) ??
            (document.id === demoDocuments[0].id
              ? sampleContent
              : `---\nid: ${document.id}\ntitle: "${document.title}"\ncreated_at: 2026-11-24T10:00:00+08:00\nupdated_at: 2026-11-25T09:00:00+08:00\ntags: []\nstatus: draft\n---\n\n# ${document.title}\n\n${document.preview.replace("……", "。")}`);
        }
        if (loadId !== documentLoadRef.current) return;
        setContent(markdown);
        setBaseHash(contentHash || (await digest(markdown)));
        if (loadId !== documentLoadRef.current) return;
        setSaveState("saved");
      } catch (cause) {
        setError(cause instanceof Error ? cause.message : String(cause));
        setSaveState("error");
      }
    },
    [setBaseHash, setContent, setError, setSaveState],
  );

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
    return () => {
      cancelled = true;
    };
  }, [setDocuments, setError, setLoading, setSelectedId, setShowDemoBanner, setWorkspace]);

  useEffect(() => {
    if (!selected) return;
    void loadDocument(selected);
  }, [selectedId, loadDocument]);

  useEffect(() => {
    if (!selected || saveState !== "unsaved") return;
    const snapshot = content;
    const requestId = ++saveRequestRef.current;
    const timer = window.setTimeout(async () => {
      if (saveInFlightRef.current) return;
      saveInFlightRef.current = true;
      setSaveState("saving");
      try {
        if (isTauri()) {
          const updated = await invoke<Document>("save_document", {
            path: selected.path,
            content: snapshot,
            baseHash,
          });
          setDocuments((current) =>
            current.map((document) => (document.id === updated.id ? updated : document)),
          );
        } else {
          localStorage.setItem(`creatorrelay.demo.content.${selected.id}`, snapshot);
          const updated = {
            ...selected,
            title: titleFromMarkdown(snapshot, selected.title),
            preview:
              bodyFromMarkdown(snapshot)
                .split("\n")
                .find((line) => line.trim() && !line.startsWith("#"))
                ?.slice(0, 120) ?? "",
            updatedAt: new Date().toISOString(),
            wordCount: countWords(snapshot),
            charCount: bodyFromMarkdown(snapshot).replace(/\s/gu, "").length,
          };
          const all = getStoredDocuments() ?? demoDocuments;
          const next = all.map((document) => (document.id === selected.id ? updated : document));
          localStorage.setItem("creatorrelay.demo.documents", JSON.stringify(next));
          setDocuments((current) =>
            current.map((document) => (document.id === selected.id ? updated : document)),
          );
        }
        const savedHash = await digest(snapshot);
        setBaseHash(savedHash);
        setSaveState(contentRef.current === snapshot ? "saved" : "unsaved");
        if (requestId === saveRequestRef.current) {
          showToast("已保存到工作区", 1800);
        }
      } catch (cause) {
        setSaveState("error");
        setError(cause instanceof Error ? cause.message : String(cause));
      } finally {
        saveInFlightRef.current = false;
        if (contentRef.current !== snapshot) setSaveState("unsaved");
      }
    }, 650);
    return () => window.clearTimeout(timer);
  }, [
    baseHash,
    content,
    saveState,
    selected,
    setBaseHash,
    setDocuments,
    setError,
    setSaveState,
    showToast,
  ]);

  useEffect(() => {
    if (!selected || isTauri()) return;
    const saved = localStorage.getItem(`creatorrelay.demo.content.${selected.id}`);
    if (!saved) {
      localStorage.setItem(
        `creatorrelay.demo.content.${selected.id}`,
        selected.id === demoDocuments[0].id
          ? sampleContent
          : `---\nid: ${selected.id}\ntitle: "${selected.title}"\ncreated_at: 2026-11-24T10:00:00+08:00\nupdated_at: 2026-11-25T09:00:00+08:00\ntags: []\nstatus: draft\n---\n\n# ${selected.title}\n\n${selected.preview.replace("……", "。")}`,
      );
    }
  }, [selected]);

  useEffect(() => {
    if (!selected || !isTauri()) return;
    invoke<Revision[]>("list_revisions", { path: selected.path })
      .then(setRevisions)
      .catch(() => setRevisions([]));
  }, [selected, saveState, setRevisions]);

  const chooseWorkspace = useCallback(async () => {
    if (saveState !== "saved" || saveInFlightRef.current) {
      showToast("请等待当前文章保存完成后再切换工作区", 2200);
      return;
    }
    setError("");
    try {
      if (!isTauri()) {
        showToast("请在桌面应用中选择真实工作区", 2200);
        return;
      }
      const path = await open({
        directory: true,
        multiple: false,
        title: "选择 CreatorRelay 工作区",
      });
      if (typeof path !== "string") return;
      const next = await invoke<WorkspaceInfo>("set_workspace_path", { path });
      setWorkspace(next);
      const items = await invoke<Document[]>("list_documents", { query: null });
      setDocuments(items);
      setSelectedId(items[0]?.id ?? "");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  }, [saveState, setDocuments, setError, setSelectedId, setWorkspace, showToast]);

  const selectDocumentSafely = useCallback(
    async (document: Document) => {
      if (document.id === selectedId) return true;
      if (saveState === "error") {
        showToast("当前文章未能保存，请先处理保存错误再切换", 2200);
        return false;
      }
      if (saveState === "saving" || saveInFlightRef.current) {
        showToast("正在保存当前文章，请稍后再切换", 1800);
        return false;
      }
      if (saveState === "unsaved" && selected) {
        const snapshot = contentRef.current;
        saveInFlightRef.current = true;
        setSaveState("saving");
        try {
          if (isTauri()) {
            const updated = await invoke<Document>("save_document", {
              path: selected.path,
              content: snapshot,
              baseHash,
            });
            setDocuments((current) =>
              current.map((item) => (item.id === updated.id ? updated : item)),
            );
          } else {
            localStorage.setItem(`creatorrelay.demo.content.${selected.id}`, snapshot);
            const updated = {
              ...selected,
              title: titleFromMarkdown(snapshot, selected.title),
              preview:
                bodyFromMarkdown(snapshot)
                  .split("\n")
                  .find((line) => line.trim() && !line.startsWith("#"))
                  ?.slice(0, 120) ?? "",
              updatedAt: new Date().toISOString(),
              wordCount: countWords(snapshot),
              charCount: bodyFromMarkdown(snapshot).replace(/\s/gu, "").length,
            };
            const all = getStoredDocuments() ?? demoDocuments;
            localStorage.setItem(
              "creatorrelay.demo.documents",
              JSON.stringify(all.map((item) => (item.id === selected.id ? updated : item))),
            );
            setDocuments((current) =>
              current.map((item) => (item.id === updated.id ? updated : item)),
            );
          }
          setBaseHash(await digest(snapshot));
          if (contentRef.current !== snapshot) {
            setSaveState("unsaved");
            setError("文章在保存期间又有新修改，请等待保存完成后再切换。");
            return false;
          }
          setSaveState("saved");
          setSelectedId(document.id);
          return true;
        } catch (cause) {
          setSaveState("error");
          setError(cause instanceof Error ? cause.message : String(cause));
          return false;
        } finally {
          saveInFlightRef.current = false;
        }
      }
      setSelectedId(document.id);
      return true;
    },
    [
      baseHash,
      saveState,
      selected,
      selectedId,
      setBaseHash,
      setDocuments,
      setError,
      setSaveState,
      setSelectedId,
      showToast,
    ],
  );

  const createDocument = useCallback(async () => {
    if (!workspace) {
      setError("请先选择一个工作区");
      setIsCreating(false);
      return null;
    }
    if (saveState !== "saved" || saveInFlightRef.current) {
      showToast("请等待当前文章保存完成后再新建文档", 2200);
      return null;
    }
    const title = newTitle.trim();
    if (!title) return null;
    try {
      let created: Document;
      if (isTauri()) {
        created = await invoke<Document>("create_document", { title });
      } else {
        const id = `article-${crypto.randomUUID()}`;
        created = {
          id,
          title,
          path: `articles/${id}/article.md`,
          updatedAt: new Date().toISOString(),
          preview: "开始写下你的想法……",
          wordCount: 0,
          charCount: 0,
          tags: [],
        };
        const all = getStoredDocuments() ?? demoDocuments;
        localStorage.setItem("creatorrelay.demo.documents", JSON.stringify([created, ...all]));
        localStorage.setItem(
          `creatorrelay.demo.content.${id}`,
          `---\nid: ${id}\ntitle: "${title}"\ncreated_at: ${new Date().toISOString()}\nupdated_at: ${new Date().toISOString()}\ntags: []\nstatus: draft\n---\n\n# ${title}\n\n`,
        );
      }
      setNewTitle("");
      setIsCreating(false);
      await loadDocuments();
      setSelectedId(created.id);
      setViewMode("write");
      return created;
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
      return null;
    }
  }, [
    loadDocuments,
    newTitle,
    saveState,
    setError,
    setIsCreating,
    setNewTitle,
    setSelectedId,
    setViewMode,
    showToast,
    workspace,
  ]);

  const restoreRevision = useCallback(
    async (revision: Revision) => {
      if (!selected) return;
      if (saveState !== "saved" || saveInFlightRef.current) {
        showToast("请等待当前文章保存完成后再恢复历史版本", 2200);
        return;
      }
      try {
        if (isTauri()) {
          const updated = await invoke<Document>("restore_revision", {
            path: selected.path,
            revisionId: revision.id,
          });
          await loadDocuments();
          await loadDocument(updated);
        } else {
          showToast("演示模式下仅展示版本记录，桌面端可恢复完整历史版本", 2500);
        }
        setShowHistory(false);
      } catch (cause) {
        setError(cause instanceof Error ? cause.message : String(cause));
      }
    },
    [loadDocument, loadDocuments, saveState, selected, setError, setShowHistory, showToast],
  );

  return {
    workspace,
    documents,
    selected,
    selectedId,
    content,
    updateContent,
    query,
    setQuery,
    saveState,
    viewMode,
    setViewMode,
    showHistory,
    setShowHistory,
    revisions,
    isCreating,
    setIsCreating,
    newTitle,
    setNewTitle,
    loading,
    error,
    setError,
    notice,
    setNotice,
    sidebarOpen,
    setSidebarOpen,
    showDemoBanner,
    setShowDemoBanner,
    body,
    words,
    chars,
    showToast,
    loadDocuments,
    chooseWorkspace,
    selectDocumentSafely,
    createDocument,
    restoreRevision,
  };
}

export type WorkspaceController = ReturnType<typeof useWorkspaceDocuments>;
