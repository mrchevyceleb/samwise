import { unified, type Processor } from "unified";
import remarkParse from "remark-parse";
import remarkGfm from "remark-gfm";
import remarkRehype from "remark-rehype";
import rehypeStringify from "rehype-stringify";
import rehypeRaw from "rehype-raw";
import rehypeSlug from "rehype-slug";
import rehypeShiki from "@shikijs/rehype";

let processor: any = null;

async function getProcessor() {
  if (!processor) {
    processor = unified()
      .use(remarkParse)
      .use(remarkGfm)
      .use(remarkRehype, { allowDangerousHtml: true })
      .use(rehypeRaw)
      .use(rehypeShiki, {
        theme: 'vitesse-dark',
        lazy: true,
        addLanguageClass: true,
      })
      .use(rehypeSlug)
      .use(rehypeStringify);
  }
  return processor;
}

/** Wrap shiki <pre> blocks with a header containing language label and copy button */
function postProcessCodeBlocks(html: string): string {
  return html.replace(
    /<pre class="shiki([^"]*)"([^>]*)>([\s\S]*?)<\/pre>/g,
    (_match, classes: string, attrs: string, inner: string) => {
      const langMatch = classes.match(/language-(\S+)/);
      const lang = langMatch ? langMatch[1] : '';

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
        + `</div><pre class="shiki${classes}"${attrs}>${inner}</pre></div>`;
    }
  );
}

export async function renderMarkdown(content: string): Promise<string> {
  const proc = await getProcessor();
  const result = await proc.process(content);
  return postProcessCodeBlocks(String(result));
}
