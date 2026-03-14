# Guided Secrets UX - Feature Spec

## Problem

Users shouldn't need to memorize env var names, know framework prefixes, or hunt through service dashboards to find their keys. The IDE should guide them through the entire process.

## Current State

The env vars panel (`src/lib/components/preview/EnvVarsPanel.svelte`) currently supports:

- **Raw key-value inputs** with manual key entry and password-masked values
- **Auto-prefix hints** - typing a base key like `SUPABASE_URL` shows a hint that `NEXT_PUBLIC_`, `VITE_`, `REACT_APP_` variants will be set automatically
- **`.env` file scanning** - on project open, scans for `.env`/`.env.local` files and extracts key names as "suggested keys" (values are NOT imported, only key names)
- **Suggested key chips** - detected keys appear as clickable chips to quickly add them
- **localStorage persistence** - env vars are saved/loaded per project path via `localStorage`
- **Apply & Restart** - button to save vars and restart the preview server with the new env injected
- **Framework prefix expansion** - `envVarsToMap()` in `preview.svelte.ts` expands base keys into all framework prefix variants when passing to the Rust backend

The store lives in `src/lib/stores/preview.svelte.ts`. The Rust backend receives the expanded env map via the `preview_open` command.

---

## Feature: Guided Secrets UX

### 1. Guided Service Detection

Scan the project's `package.json` dependencies (both `dependencies` and `devDependencies`) to auto-detect which services the project uses. Show a dedicated section for each detected service with pre-configured fields.

**Detection mapping:**

| Dependency | Service | Fields to Show |
|---|---|---|
| `@supabase/supabase-js`, `@supabase/ssr` | Supabase | URL, Anon Key, Service Role Key |
| `stripe`, `@stripe/stripe-js` | Stripe | Secret Key, Publishable Key, Webhook Secret |
| `openai` | OpenAI | API Key |
| `@anthropic-ai/sdk` | Anthropic | API Key |
| `@google/generative-ai` | Google AI | API Key |
| `firebase`, `firebase-admin` | Firebase | API Key, Auth Domain, Project ID |
| `@clerk/nextjs`, `@clerk/clerk-react` | Clerk | Publishable Key, Secret Key |
| `@auth0/auth0-react`, `auth0` | Auth0 | Domain, Client ID, Client Secret |
| `resend` | Resend | API Key |
| `@sentry/nextjs`, `@sentry/react` | Sentry | DSN |
| `@upstash/redis` | Upstash Redis | URL, Token |
| `@planetscale/database` | PlanetScale | Database URL |
| `pg`, `postgres` | PostgreSQL | Database URL |
| `@prisma/client` | Prisma | Database URL |
| `drizzle-orm` | Drizzle | Database URL |
| `@aws-sdk/*` | AWS | Access Key ID, Secret Access Key, Region |
| `twilio` | Twilio | Account SID, Auth Token |
| `@sendgrid/mail` | SendGrid | API Key |

**Implementation:**
- Add a Tauri command `preview_detect_services` that reads `package.json` and returns a list of detected services
- Call this during `openProject()` alongside the existing `.env` scan
- Store detected services in the preview store as `detectedServices`

### 2. Known Service Fields

Each detected service renders as a collapsible group with:

- **Service icon/logo** (small inline SVG or emoji fallback)
- **Service name** as the group header (e.g., "Supabase", "Stripe")
- **Labeled inputs** for each field:
  - Friendly label (e.g., "Anon Key" not `SUPABASE_ANON_KEY`)
  - The actual env var name shown in muted mono text below the label
  - Password-masked input with show/hide toggle (reuse existing pattern)
- **Help tooltip** on each field with:
  - One-line description of what this value is
  - Direct clickable link to the service dashboard page where the user can copy the value

**Tooltip links by service:**

| Service | Field | Dashboard Link |
|---|---|---|
| Supabase | URL | `https://supabase.com/dashboard/project/_/settings/api` |
| Supabase | Anon Key | `https://supabase.com/dashboard/project/_/settings/api` |
| Supabase | Service Role Key | `https://supabase.com/dashboard/project/_/settings/api` |
| Stripe | Secret Key | `https://dashboard.stripe.com/apikeys` |
| Stripe | Publishable Key | `https://dashboard.stripe.com/apikeys` |
| OpenAI | API Key | `https://platform.openai.com/api-keys` |
| Anthropic | API Key | `https://console.anthropic.com/settings/keys` |
| Google AI | API Key | `https://aistudio.google.com/apikey` |
| Clerk | Keys | `https://dashboard.clerk.com` |
| Resend | API Key | `https://resend.com/api-keys` |
| Sentry | DSN | `https://sentry.io/settings/projects/` |
| Twilio | Credentials | `https://console.twilio.com` |
| SendGrid | API Key | `https://app.sendgrid.com/settings/api_keys` |

**Data structure:**

```typescript
interface KnownService {
  id: string;             // e.g., "supabase"
  name: string;           // e.g., "Supabase"
  icon: string;           // inline SVG string or emoji
  fields: KnownServiceField[];
}

interface KnownServiceField {
  label: string;          // e.g., "Anon Key"
  envKey: string;         // e.g., "SUPABASE_ANON_KEY"
  helpText: string;       // e.g., "The public anonymous key for client-side access"
  dashboardUrl: string;   // e.g., "https://supabase.com/dashboard/project/_/settings/api"
  isPublic: boolean;      // true = should get framework prefixes, false = server-only
}
```

The `isPublic` flag controls auto-prefixing. Server-only keys (like `SUPABASE_SERVICE_ROLE_KEY`, `STRIPE_SECRET_KEY`) should NOT get `NEXT_PUBLIC_` etc. prefixes.

### 3. LLM API Key Dropdown

A special section (always shown, not dependent on detection) for selecting an LLM provider:

- **Dropdown** with options: OpenAI, Anthropic, Google AI, Mistral, Groq, Together AI, Fireworks AI, Perplexity, Cohere
- Selecting a provider auto-fills the env var name (e.g., selecting "OpenAI" sets key to `OPENAI_API_KEY`)
- Single input field for the API key value
- Help link updates based on selection
- Allow adding multiple LLM providers (button to add another)

**Provider-to-key mapping:**

| Provider | Env Key | Dashboard URL |
|---|---|---|
| OpenAI | `OPENAI_API_KEY` | `https://platform.openai.com/api-keys` |
| Anthropic | `ANTHROPIC_API_KEY` | `https://console.anthropic.com/settings/keys` |
| Google AI | `GOOGLE_AI_API_KEY` | `https://aistudio.google.com/apikey` |
| Mistral | `MISTRAL_API_KEY` | `https://console.mistral.ai/api-keys` |
| Groq | `GROQ_API_KEY` | `https://console.groq.com/keys` |
| Together AI | `TOGETHER_API_KEY` | `https://api.together.xyz/settings/api-keys` |
| Fireworks AI | `FIREWORKS_API_KEY` | `https://fireworks.ai/api-keys` |
| Perplexity | `PERPLEXITY_API_KEY` | `https://www.perplexity.ai/settings/api` |
| Cohere | `COHERE_API_KEY` | `https://dashboard.cohere.com/api-keys` |

### 4. Import Options

Three import methods, accessible via an "Import" dropdown button in the panel header.

#### 4a. Import .env File

- **File picker** button that opens a native file dialog (via Tauri `dialog.open()`) filtered to `.env*` files
- **Drag-and-drop** zone (the entire env panel acts as a drop target)
- Parses the file and imports BOTH keys AND values (unlike the current scan which only imports key names)
- On import, merges with existing vars. If a key already exists, prompt to overwrite or skip.
- Supports standard `.env` format: `KEY=value`, `KEY="quoted value"`, comments (`#`), empty lines

#### 4b. Import JSON

- **Paste JSON** button opens a small textarea modal
- User pastes a JSON object like `{"SUPABASE_URL": "https://...", "SUPABASE_ANON_KEY": "eyJ..."}`
- Validates JSON, shows error if invalid
- Same merge behavior as .env import

#### 4c. Doppler Integration

- **Connect to Doppler** button
- Requires Doppler CLI to be installed (`doppler` command available)
- Flow:
  1. Check if `doppler` CLI is available (run `doppler --version` via Tauri command)
  2. If not installed, show link to install page
  3. If installed, run `doppler secrets download --no-file --format json` in the project directory
  4. Parse JSON output and import all key-value pairs
  5. If Doppler isn't configured for the project, show helpful message about running `doppler setup`
- The Doppler tab in settings (`src/lib/components/settings/DopplerTab.svelte`) may already have some of this. Check for reusable logic.

### 5. Custom Variables Section

Below the guided sections, keep a "Custom Variables" section that works like the current raw key-value interface:

- Header: "Custom Variables"
- Same "Add Variable" button and key=value row pattern as today
- This is the escape hatch for any env var not covered by known services
- Existing vars that don't match any known service field automatically appear here

### 6. Auto-Prefix Magic

Expand the current prefix system:

- **Current behavior (keep):** `envVarsToMap()` expands base keys into all framework variants
- **New: Smart prefix based on detected framework:**
  - Detect the framework from `package.json` (Next.js, Vite, Nuxt, etc.)
  - Show the user which specific prefix will be used: "Will be set as `NEXT_PUBLIC_SUPABASE_URL`"
  - Still set ALL prefix variants for compatibility, but highlight the primary one
- **New: Respect `isPublic` flag:** Server-only keys (database URLs, secret keys) should NOT get public prefixes. Only keys marked `isPublic: true` in the service definition get expanded.
- **New: Strip prefix on input:** If user types `NEXT_PUBLIC_SUPABASE_URL`, auto-strip the prefix and store as `SUPABASE_URL` (the expansion handles the rest)

### 7. Panel Layout (Top to Bottom)

```
+--------------------------------------------------+
| Environment Variables                    [Import v] [X] |
+--------------------------------------------------+
| [Detected: .env chips for unrecognized keys]      |
+--------------------------------------------------+
| > Supabase (detected)                             |
|   Project URL    [________________________] [?]   |
|   Anon Key       [________________________] [?]   |
|   Service Key    [________________________] [?]   |
+--------------------------------------------------+
| > Stripe (detected)                               |
|   Secret Key     [________________________] [?]   |
|   Publishable    [________________________] [?]   |
+--------------------------------------------------+
| > LLM Provider                                    |
|   [OpenAI v]     [________________________] [?]   |
|   [+ Add another provider]                        |
+--------------------------------------------------+
| > Custom Variables                                |
|   [KEY_____] = [VALUE_______________] [x]         |
|   [+ Add Variable]                                |
+--------------------------------------------------+
|              [Apply & Restart]                    |
+--------------------------------------------------+
```

### 8. Data Model Changes

In `preview.svelte.ts`, add:

```typescript
interface DetectedService {
  service: KnownService;
  values: Record<string, string>;  // envKey -> value
}

// New state
let detectedServices = $state<DetectedService[]>([]);
let detectedFramework = $state<string | null>(null);
let llmProviders = $state<{ provider: string; key: string }[]>([]);
```

Persistence: all values (guided + custom) save to localStorage under the same key, merged into a flat `EnvVar[]` array. The guided UI is just a view layer on top of the same data.

### 9. Implementation Order

1. **Define known services data** - Create `src/lib/data/known-services.ts` with all service definitions, field mappings, and dashboard URLs
2. **Add Tauri command for detection** - `preview_detect_services` reads `package.json` and returns matched services
3. **Update preview store** - Add `detectedServices`, `detectedFramework`, `llmProviders` state. Update `openProject()` to call detection.
4. **Build service group component** - `ServiceGroup.svelte` renders a collapsible group of labeled inputs with tooltips
5. **Build LLM provider dropdown** - `LlmProviderSelect.svelte` with provider dropdown and key input
6. **Build import dropdown** - `ImportMenu.svelte` with .env file picker, JSON paste modal, Doppler connect
7. **Refactor EnvVarsPanel** - Compose the new components into the panel, keeping custom vars at the bottom
8. **Update `envVarsToMap()`** - Respect `isPublic` flag, add framework-specific primary prefix display
9. **Test end-to-end** - Open a real project, verify detection, enter values, apply & restart

### 10. Files to Create/Modify

| File | Action |
|---|---|
| `src/lib/data/known-services.ts` | Create - service definitions and field mappings |
| `src/lib/components/preview/ServiceGroup.svelte` | Create - collapsible service field group |
| `src/lib/components/preview/LlmProviderSelect.svelte` | Create - LLM provider dropdown |
| `src/lib/components/preview/ImportMenu.svelte` | Create - import dropdown (env, JSON, Doppler) |
| `src/lib/components/preview/EnvVarsPanel.svelte` | Modify - compose new components, restructure layout |
| `src/lib/stores/preview.svelte.ts` | Modify - add detection state, update persistence |
| `src-tauri/src/commands/preview.rs` | Modify - add `preview_detect_services` command |
| `src-tauri/src/lib.rs` | Modify - register new command |
