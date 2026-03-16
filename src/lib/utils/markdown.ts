import { unified } from "unified";
import remarkParse from "remark-parse";
import remarkGfm from "remark-gfm";
import remarkRehype from "remark-rehype";
import rehypeStringify from "rehype-stringify";
import rehypeRaw from "rehype-raw";
import rehypeSlug from "rehype-slug";

let processor: any = null;

async function getProcessor() {
  if (!processor) {
    processor = unified()
      .use(remarkParse)
      .use(remarkGfm)
      .use(remarkRehype, { allowDangerousHtml: true })
      .use(rehypeRaw)
      .use(rehypeSlug)
      .use(rehypeStringify);
  }
  return processor;
}

/** Wrap <pre><code> blocks with a header containing language label and copy button */
function postProcessCodeBlocks(html: string): string {
  return html.replace(
    /<pre><code class="language-(\w+)">([\s\S]*?)<\/code><\/pre>/g,
    (_match, lang: string, inner: string) => {
      const plainText = inner.replace(/<[^>]+>/g, '')
        .replace(/&lt;/g, '<')
        .replace(/&gt;/g, '>')
        .replace(/&amp;/g, '&')
        .replace(/&quot;/g, '"')
        .replace(/&#39;/g, "'");

      const escapedCode = plainText
        .replace(/&/g, '&amp;')
        .replace(/"/g, '&quot;');

      return `<div class="code-block-wrapper"><div class="code-block-header" style="padding: 6px 12px; display: flex; align-items: center; justify-content: space-between;">`
        + `<span class="code-block-lang">${lang}</span>`
        + `<button class="code-block-copy" data-code="${escapedCode}" style="padding: 2px 8px;">Copy</button>`
        + `</div><pre><code class="language-${lang}">${inner}</code></pre></div>`;
    }
  );
}

export async function renderMarkdown(content: string): Promise<string> {
  const proc = await getProcessor();
  const result = await proc.process(content);
  return postProcessCodeBlocks(String(result));
}
