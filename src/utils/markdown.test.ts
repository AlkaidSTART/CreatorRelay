import { describe, expect, it } from "vitest";
import { bodyFromMarkdown, countWords, titleFromMarkdown } from "./markdown";

describe("markdown helpers", () => {
  it("removes frontmatter before returning the document body", () => {
    const markdown = "---\ntitle: Sample\ntags: []\n---\n\n# Body\n";

    expect(bodyFromMarkdown(markdown)).toBe("# Body\n");
  });

  it("keeps content intact when frontmatter is missing or incomplete", () => {
    expect(bodyFromMarkdown("# Body")).toBe("# Body");
    expect(bodyFromMarkdown("---\ntitle: Sample\n# Body")).toBe("---\ntitle: Sample\n# Body");
  });

  it("reads the title from frontmatter, then a Markdown heading, then the fallback", () => {
    expect(titleFromMarkdown("---\ntitle: \"From metadata\"\n---\n# Body", "Fallback")).toBe("From metadata");
    expect(titleFromMarkdown("# From heading", "Fallback")).toBe("From heading");
    expect(titleFromMarkdown("Body without a heading", "Fallback")).toBe("Fallback");
  });

  it("counts words in the body without counting frontmatter", () => {
    expect(countWords("---\ntitle: Not a word\n---\nCreatorRelay is local first.")).toBe(4);
  });
});
