# Nodalix Assistant Architecture

Nodalix Assistant is the local-first intelligence layer for Nodalix OS. It is not a chatbot bolted onto the desktop; it is a system layer made of intents, permissions, context connectors, providers and UI surfaces.

## Principles

- Local-first by default.
- Sensitive data access requires explicit permissions.
- External providers must not receive mail, contacts, files, messages or screen context without user consent.
- Apps declare capabilities through manifests instead of being hardcoded in the assistant.
- Every sensitive access is auditable.
- Every write or destructive action requires confirmation.

## Components

### Assistant Core

Location: `crates/nodalix-assistant-core`

Responsibilities:

- classify user queries
- discover intents
- check permissions before sensitive context access
- search the local context index
- return answers, sources and suggested actions
- abstract model providers

The current binary is development-oriented:

```bash
nodalix-assistant status
nodalix-assistant ask "dónde vive jose"
nodalix-assistant grant nodalix-assistant contacts.read
nodalix-assistant index-clear
```

### Intents

Location: `config/nodalix/intents/*.intent.json`

Apps declare actions with:

- `id`
- `name`
- `description`
- `permissions`
- examples
- confirmation requirements
- background capability
- whether the intent modifies user data

Initial manifests cover:

- `contacts.findPerson`
- `contacts.updateAddress`
- `mail.search`
- `mail.summarize`
- `mail.replyDraft`
- `mail.extractEvent`
- `calendar.createEvent`
- `reminders.createReminder`
- `settings.openPanel`
- `system.lock`
- `system.suspend`
- `system.poweroffConfirm`
- `bar.restart`
- `controlCenter.open`

### Permission Broker

Permissions are stored locally:

```text
~/.config/nodalix/privacy/permissions.json
```

Sensitive access is audited as JSON lines:

```text
~/.local/share/nodalix/privacy/audit.log
```

Initial permission categories:

- `contacts.read`
- `contacts.write`
- `messages.read`
- `mail.read`
- `mail.writeDraft`
- `mail.send`
- `calendar.read`
- `calendar.write`
- `reminders.read`
- `reminders.write`
- `files.read`
- `files.write`
- `screen.read`
- `system.actions`

### Context Index

Current path:

```text
~/.local/share/nodalix/assistant/context-index.json
```

The first implementation is intentionally simple: JSON documents with connector, title, body, entities and internal source references. It supports exact, token and fuzzy matching. SQLite/FTS5 or vector storage can replace the backend without changing the public contract.

No connector indexes anything automatically yet.

### Providers

The provider interface supports:

- `chat`
- `summarize`
- `classify_intent`

The current provider is `MockProvider`, which is local and deterministic for development. Future providers can include Ollama, llama.cpp, OpenAI-compatible APIs and custom HTTP providers behind policy checks.

## Query Flow

For “¿Dónde vive José?”:

1. provider classifies the intent as `contacts.findPerson`
2. Assistant asks Permission Broker for `contacts.read`
3. if denied, it returns a permission request and audits the access
4. if granted, it searches Context Index for likely contacts
5. entity resolver ranks candidates
6. response includes confidence, internal sources and suggested actions

## Integrations

### Command Bar

Command Bar keeps local apps/actions first. Natural-language questions get a local “Preguntar a Nodalix Assistant” action before web search.

### Settings

Settings has an initial “Intelligence” page with:

- assistant status
- permissions path
- audit path
- index path
- clear-index action
- policy notes

### Mail

Nodalix Mail is represented by intents now. The next phase should add a mail connector that reads only configured accounts and only after `mail.read`.

### Contacts

Contacts are represented by intents and expected index documents. The next phase should add local contacts storage/import and a connector that emits `contacts` documents.

## Phase Plan

### Phase 0 implemented

- assistant core contracts
- manifest-based intents
- permission broker
- audit log
- local context index
- mock provider
- Command Bar integration
- Settings entry point

### Phase 1 next

- local contacts connector
- local mail test connector
- sample event extraction
- UI confirmation flow for grants and writes
- assistant conversation window
