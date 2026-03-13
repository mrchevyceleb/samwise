<script lang="ts">
  interface Props {
    diff?: string;
  }

  let { diff = '' }: Props = $props();

  interface DiffLine {
    type: 'added' | 'removed' | 'context' | 'header';
    content: string;
    lineNum?: number;
  }

  let parsedLines = $derived<DiffLine[]>(parseDiff(diff));

  function parseDiff(raw: string): DiffLine[] {
    if (!raw) return [];
    const lines: DiffLine[] = [];
    let lineNum = 0;
    for (const line of raw.split('\n')) {
      if (line.startsWith('@@')) {
        // Parse line number from hunk header
        const match = line.match(/@@ -\d+(?:,\d+)? \+(\d+)/);
        if (match) lineNum = parseInt(match[1], 10) - 1;
        lines.push({ type: 'header', content: line });
      } else if (line.startsWith('+') && !line.startsWith('+++')) {
        lineNum++;
        lines.push({ type: 'added', content: line.substring(1), lineNum });
      } else if (line.startsWith('-') && !line.startsWith('---')) {
        lines.push({ type: 'removed', content: line.substring(1) });
      } else if (line.startsWith('diff ') || line.startsWith('index ') || line.startsWith('---') || line.startsWith('+++')) {
        lines.push({ type: 'header', content: line });
      } else {
        lineNum++;
        lines.push({ type: 'context', content: line.startsWith(' ') ? line.substring(1) : line, lineNum });
      }
    }
    return lines;
  }

  function lineBackground(type: string): string {
    switch (type) {
      case 'added': return 'rgba(63, 185, 80, 0.12)';
      case 'removed': return 'rgba(248, 81, 73, 0.12)';
      case 'header': return 'rgba(88, 166, 255, 0.08)';
      default: return 'transparent';
    }
  }

  function lineColor(type: string): string {
    switch (type) {
      case 'added': return 'var(--accent-green)';
      case 'removed': return 'var(--accent-red)';
      case 'header': return 'var(--accent-blue)';
      default: return 'var(--text-primary)';
    }
  }

  function linePrefix(type: string): string {
    switch (type) {
      case 'added': return '+';
      case 'removed': return '-';
      default: return ' ';
    }
  }
</script>

<div style="height: 100%; overflow: auto; background: var(--bg-primary); font-family: var(--font-mono); font-size: 12px; line-height: 1.5;">
  {#if parsedLines.length === 0}
    <div style="display: flex; align-items: center; justify-content: center; height: 100%; color: var(--text-muted); font-size: 12px;">
      No diff to display
    </div>
  {:else}
    {#each parsedLines as line, i}
      <div style="display: flex; min-height: 20px; background: {lineBackground(line.type)}; border-left: 3px solid {line.type === 'added' ? 'var(--accent-green)' : line.type === 'removed' ? 'var(--accent-red)' : 'transparent'};">
        <span style="width: 48px; text-align: right; padding-right: 8px; color: var(--text-muted); opacity: 0.5; flex-shrink: 0; user-select: none;">
          {line.lineNum || ''}
        </span>
        <span style="width: 16px; text-align: center; color: {lineColor(line.type)}; flex-shrink: 0; user-select: none;">
          {line.type === 'header' ? '' : linePrefix(line.type)}
        </span>
        <pre style="flex: 1; margin: 0; white-space: pre-wrap; word-break: break-all; color: {lineColor(line.type)}; padding-right: 8px;">{line.content}</pre>
      </div>
    {/each}
  {/if}
</div>
