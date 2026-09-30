import assert from 'node:assert/strict';
import { after, before, test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { createServer } from 'vite';

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
      svelte({ configFile: false })
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
  ['review failure before deploy', 'review', { samwise_review_merge_status: 'failed' }, 'Retry Review & Merge'],
  ['fixes_needed review failure', 'fixes_needed', { samwise_review_merge_status: 'failed' }, 'Retry Review & Merge'],
  ['review failure with null deploy', 'review', { samwise_review_merge_status: 'failed', samwise_merge_deploy_status: null }, 'Retry Review & Merge'],
  ['review failure after previous deploy', 'review', { samwise_review_merge_status: 'failed', samwise_merge_deploy_status: 'succeeded' }, 'Retry Review & Merge'],
  ['deploy failure', 'fixes_needed', { samwise_merge_deploy_status: 'failed' }, 'Retry Review & Merge'],
  ['approved PR', 'approved', null, 'Review & Merge'],
  ['review without pipeline failure', 'review', null, 'Mark Done'],
  ['review failure without a PR', 'review', { samwise_review_merge_status: 'failed' }, 'Mark Done', null],
  ['queued review stays disabled', 'approved', { samwise_review_merge_status: 'requested' }, 'Sam Queued'],
  ['running review stays disabled', 'approved', { samwise_review_merge_status: 'running' }, 'Sam Reviewing...']
];

for (const name of ['KanbanCard', 'TaskDetail']) {
  for (const [scenario, status, context, label, pr_url = 'https://github.com/example/app/pull/1'] of cases) {
    test(`${name}: ${scenario}`, () => {
      const task = {
        id: 'review-retry-test', title: 'Review retry regression', description: null,
        status, context, pr_url, priority: 'medium', assignee: 'agent',
        created_at: '2026-08-30T00:00:00Z', updated_at: '2026-08-30T00:00:00Z',
        review_summary: 'Review result available.'
      };
      const { body } = render(components[name], { props: { task, onOpen() {}, onClose() {} } });
      const buttons = [...body.matchAll(/<button\b([^>]*)>([\s\S]*?)<\/button>/g)]
        .map(([, attributes, content]) => ({
          attributes, label: content.replace(/<!--[^]*?-->/g, '').replace(/&amp;/g, '&').trim()
        }));
      const action = buttons.find((button) => button.label === label);
      assert.ok(action, `Expected ${label} in ${name}; got ${buttons.map((button) => button.label).join(', ')}`);
      if (label !== 'Mark Done') {
        assert.ok(!buttons.some((button) => button.label === 'Mark Done'), 'Must not offer Mark Done for this pipeline action');
      }
      const busy = ['Sam Queued', 'Sam Reviewing...'].includes(label);
      assert.equal(/\bdisabled\b/.test(action.attributes), busy);
    });
  }
}
