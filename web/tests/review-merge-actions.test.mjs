import assert from 'node:assert/strict';
import { after, before, test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { createServer } from 'vite';
import svelteConfig from '../svelte.config.js';

let server;
let render;
const components = {};
before(async () => {
  server = await createServer({
    root: fileURLToPath(new URL('../', import.meta.url)),
    configFile: false,
    plugins: [
      // Rendering needs only cached comments; never contact Supabase in this test.
      {
        name: 'test-task-store',
        resolveId: (id) => id === '\0test-task-store' ? id : null,
        load: (id) => id === '\0test-task-store' ? 'export const tasksStore = { comments: {} };' : null
      },
      // Reuse app preprocessing; npm test syncs the tsconfig Vite resolves.
      svelte({ configFile: false, preprocess: svelteConfig.preprocess })
    ],
    resolve: { alias: [
      { find: '$lib/stores/tasks.svelte', replacement: '\0test-task-store' },
      { find: '$lib', replacement: fileURLToPath(new URL('../src/lib', import.meta.url)) }
    ] },
    server: { middlewareMode: true, hmr: false, watch: null },
    optimizeDeps: { noDiscovery: true, include: [] }
  });
  ({ render } = await server.ssrLoadModule('svelte/server'));
  for (const name of ['KanbanCard', 'TaskDetail']) {
    components[name] = (await server.ssrLoadModule(`/src/lib/components/${name}.svelte`)).default;
  }
});
after(async () => { await server?.close(); });

const cases = [
  ['review failure before deploy', 'review', { samwise_review_merge_status: 'failed' }, 'Retry Review & Merge', false],
  ['fixes_needed review failure', 'fixes_needed', { samwise_review_merge_status: 'failed' }, 'Retry Review & Merge', false],
  ['review failure with null deploy', 'review', { samwise_review_merge_status: 'failed', samwise_merge_deploy_status: null }, 'Retry Review & Merge', false],
  ['review failure after previous deploy', 'review', { samwise_review_merge_status: 'failed', samwise_merge_deploy_status: 'succeeded' }, 'Retry Review & Merge', false],
  ['deploy failure', 'fixes_needed', { samwise_merge_deploy_status: 'failed' }, 'Retry Review & Merge', false],
  ['worker review failure', 'approved', { samwise_review_merge_status: 'failed' }, 'Retry Review & Merge', false],
  ['worker review blocker', 'fixes_needed', { samwise_review_merge_status: 'blocked' }, 'Retry Review & Merge', false],
  ['approved PR', 'approved', null, 'Review & Merge', false],
  ['review without pipeline failure', 'review', null, 'Mark Done', false],
  ['review failure without a PR', 'review', { samwise_review_merge_status: 'failed' }, 'Mark Done', false, null],
  ['failure without a review summary', 'review', { samwise_review_merge_status: 'failed' }, 'Retry Review & Merge', false, undefined, null],
  ['queued review stays disabled', 'approved', { samwise_review_merge_status: 'requested', samwise_merge_conflict_fix_status: 'requested' }, 'Sam Queued', true],
  ['running review stays disabled', 'approved', { samwise_review_merge_status: 'running' }, 'Sam Reviewing...', true],
  ['recover orphaned queued retry in review', 'review', { samwise_review_merge_status: 'requested' }, 'Retry Review & Merge', false],
  ['running retry in review', 'review', { samwise_review_merge_status: 'running' }, 'Sam Reviewing...', true],
  ['recover orphaned queued retry in fixes_needed', 'fixes_needed', { samwise_review_merge_status: 'requested' }, 'Retry Review & Merge', false],
  ['running retry in fixes_needed', 'fixes_needed', { samwise_review_merge_status: 'running' }, 'Sam Reviewing...', true],
  ['do not recover while deployment is active', 'review', { samwise_review_merge_status: 'requested', samwise_merge_deploy_status: 'running' }, 'Sam Queued', true]
];

for (const name of ['KanbanCard', 'TaskDetail']) {
  for (const [scenario, status, context, label, disabled, pr_url = 'https://github.com/example/app/pull/1', review_summary = 'Review result available.'] of cases) {
    test(`${name}: ${scenario}`, () => {
      const task = {
        id: 'review-retry-test', title: 'Review retry regression', description: null,
        status, context, pr_url, priority: 'medium', assignee: 'agent',
        created_at: '2026-08-30T00:00:00Z', updated_at: '2026-08-30T00:00:00Z',
        review_summary
      };
      const { body } = render(components[name], { props: { task, onOpen() {}, onClose() {} } });
      // Consume quoted attribute values as units: classes can contain > or disabled:.
      const buttons = [...body.matchAll(/<button\b((?:[^"'<>]|"[^"]*"|'[^']*')*)>([\s\S]*?)<\/button>/g)]
        .map(([, attributes, content]) => ({
          attributes: new Map([...attributes.matchAll(/([^\s=]+)(?:\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s]+)))?/g)]
            .map(([, key, doubleQuoted, singleQuoted, unquoted]) => [key, doubleQuoted ?? singleQuoted ?? unquoted ?? ''])),
          label: content.replace(/<!--[^]*?-->/g, '').replace(/&amp;/g, '&').trim()
        }));
      const actions = buttons.filter((button) => button.attributes.get('data-review-action') === 'primary');
      assert.equal(actions.length, 1, `Expected one primary review action in ${name}`);
      const [action] = actions;
      assert.equal(action.label, label);
      if (label !== 'Mark Done') {
        assert.ok(!buttons.some((button) => button.label === 'Mark Done'), 'Must not offer Mark Done for this pipeline action');
      }
      assert.equal(action.attributes.has('disabled'), disabled);
    });
  }
}
