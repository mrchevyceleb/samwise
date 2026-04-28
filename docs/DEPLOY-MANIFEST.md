# Samwise Deploy Manifest

Samwise looks for `.samwise/deploy.json` in a target repo during Merge + Deploy.
The manifest is the repo-owned source of truth for deployment commands that are
too project-specific to infer safely.

```json
{
  "rules": [
    {
      "name": "Railway tools server",
      "category": "railway",
      "paths": ["tools-server/**"],
      "commands": ["npm run tools:deploy"]
    },
    {
      "name": "Railway app server",
      "category": "railway",
      "paths": ["server/**", "Dockerfile", "railway.json"],
      "commands": ["npm run server:deploy"]
    },
    {
      "name": "Supabase",
      "paths": ["supabase/migrations/**", "supabase/functions/**"],
      "commands": ["samwise:supabase:auto"]
    }
  ]
}
```

Rules match changed PR files. `commands` run after the PR is merged, from the
repo root unless `cwd` is set to a relative path. Absolute or parent-directory
`cwd` values are rejected.

`samwise:supabase:auto` is a built-in alias. It keeps Supabase migrations and
Edge Functions on Samwise's normal automatic deploy path:

- `supabase/migrations/*.sql` runs `supabase db push`
- `supabase/functions/<name>/**` runs `supabase functions deploy <name>`

Railway deploys fail closed: if changed files imply a Railway deploy but no
manifest rule matches them, Samwise refuses to merge.
