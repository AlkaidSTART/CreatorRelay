export function bodyFromMarkdown(markdown: string) {
  if (markdown.startsWith("---\n")) {
    const end = markdown.indexOf("\n---", 4);
    if (end >= 0) return markdown.slice(end + 5).replace(/^\n/, "");
  }
  return markdown;
}

export function titleFromMarkdown(markdown: string, fallback: string) {
  const frontmatter = markdown.match(/^---\n([\s\S]*?)\n---/);
  const title = frontmatter?.[1].match(/^title:\s*["']?(.*?)["']?\s*$/m)?.[1];
  if (title) return title;
  return markdown.match(/^#\s+(.+)$/m)?.[1]?.trim() ?? fallback;
}

export function countWords(markdown: string) {
  return bodyFromMarkdown(markdown).trim().split(/\s+/u).filter(Boolean).length;
}
