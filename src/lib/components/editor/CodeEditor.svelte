<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { EditorState } from '@codemirror/state';
  import { EditorView, lineNumbers, highlightActiveLine, highlightActiveLineGutter, keymap } from '@codemirror/view';
  import { defaultKeymap, history, historyKeymap } from '@codemirror/commands';
  import { bracketMatching, foldGutter, indentOnInput } from '@codemirror/language';
  import { oneDark } from '@codemirror/theme-one-dark';
  import { javascript } from '@codemirror/lang-javascript';
  import { html } from '@codemirror/lang-html';
  import { css } from '@codemirror/lang-css';
  import { json } from '@codemirror/lang-json';
  import { python } from '@codemirror/lang-python';
  import { rust } from '@codemirror/lang-rust';
  import { markdown } from '@codemirror/lang-markdown';

  interface Props {
    content?: string;
    language?: string;
    readonly?: boolean;
    onChange?: (value: string) => void;
  }

  let { content = '', language = '', readonly = false, onChange }: Props = $props();

  let containerEl: HTMLDivElement;
  let view: EditorView | null = null;

  function getLanguageExtension(lang: string) {
    const l = lang.toLowerCase();
    if (['js', 'javascript', 'jsx', 'mjs'].includes(l)) return javascript();
    if (['ts', 'typescript', 'tsx', 'mts'].includes(l)) return javascript({ typescript: true });
    if (['svelte', 'html', 'htm', 'vue'].includes(l)) return html();
    if (['css', 'scss', 'less'].includes(l)) return css();
    if (['json', 'jsonc'].includes(l)) return json();
    if (['py', 'python'].includes(l)) return python();
    if (['rs', 'rust'].includes(l)) return rust();
    if (['md', 'markdown', 'mdx'].includes(l)) return markdown();
    if (['jsx', 'tsx'].includes(l)) return javascript({ jsx: true, typescript: l === 'tsx' });
    return null;
  }

  function detectLanguage(ext: string): string {
    const map: Record<string, string> = {
      js: 'javascript', mjs: 'javascript', cjs: 'javascript', jsx: 'jsx',
      ts: 'typescript', mts: 'typescript', tsx: 'tsx',
      svelte: 'html', html: 'html', htm: 'html', vue: 'html',
      css: 'css', scss: 'css', less: 'css',
      json: 'json', jsonc: 'json',
      py: 'python', pyw: 'python',
      rs: 'rust',
      md: 'markdown', mdx: 'markdown',
    };
    return map[ext] || ext;
  }

  onMount(() => {
    const lang = language || '';
    const langExt = getLanguageExtension(lang);

    const extensions = [
      lineNumbers(),
      highlightActiveLine(),
      highlightActiveLineGutter(),
      bracketMatching(),
      foldGutter(),
      indentOnInput(),
      history(),
      keymap.of([...defaultKeymap, ...historyKeymap]),
      oneDark,
      EditorView.theme({
        '&': {
          height: '100%',
          fontSize: '13px',
          fontFamily: "'JetBrains Mono', 'Cascadia Code', 'Fira Code', monospace",
        },
        '.cm-content': { padding: '8px 0' },
        '.cm-gutters': { background: '#0D1117', borderRight: '1px solid var(--border-default)' },
        '.cm-activeLineGutter': { background: 'rgba(255, 214, 10, 0.08)' },
        '.cm-activeLine': { background: 'rgba(255, 214, 10, 0.04)' },
        '&.cm-focused .cm-cursor': { borderLeftColor: '#FFD60A' },
        '&.cm-focused .cm-selectionBackground, ::selection': { background: 'rgba(255, 214, 10, 0.2) !important' },
      }),
    ];

    if (langExt) extensions.push(langExt);
    if (readonly) extensions.push(EditorState.readOnly.of(true));
    if (onChange) {
      extensions.push(EditorView.updateListener.of((update) => {
        if (update.docChanged) {
          onChange(update.state.doc.toString());
        }
      }));
    }

    view = new EditorView({
      state: EditorState.create({ doc: content, extensions }),
      parent: containerEl,
    });
  });

  // Update content when prop changes
  $effect(() => {
    if (view && content !== view.state.doc.toString()) {
      view.dispatch({
        changes: { from: 0, to: view.state.doc.length, insert: content },
      });
    }
  });

  onDestroy(() => {
    if (view) {
      view.destroy();
      view = null;
    }
  });

  export function getLanguageFromFilename(filename: string): string {
    const ext = filename.split('.').pop() || '';
    return detectLanguage(ext);
  }
</script>

<div bind:this={containerEl} style="height: 100%; overflow: hidden; background: #0D1117; border-radius: 4px;"></div>
