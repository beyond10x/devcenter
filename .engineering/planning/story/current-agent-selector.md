---
format: aep.planning-md/1
id: story:current-agent-selector
kind: story
status: implemented
title: Make the current agent selector prominent
summary: Make agent selection obvious without bringing back the persistent roster.
relations:
- derived_from: epic:authenticated-control-plane
- informed_by: story:agent-platform-journey
scope:
- confidence: cited
  path: frontend/e2e/devcenter.spec.ts
- confidence: cited
  path: frontend/src/features/agents/AgentsView.vue
- confidence: cited
  path: frontend/src/styles/main.css
revision: 5
---
# Story: Make the current agent selector prominent

## Outcome

An authenticated engineer can immediately identify and switch the current agent without restoring the former persistent roster sidebar.

## Acceptance

- The chat-first Agents view presents a prominent, labeled current-agent dropdown with a clear interaction target.
- The selector shows the current agent, the number available, and remains compact across desktop and mobile layouts.
- The agent icon and label align deliberately with the selector.
- Selecting an agent updates the routed URL, conversation history, and composer context.
- Browser coverage proves deep-link selection and switching, and the complete frontend accessibility and build gates pass.

## Scope

- `frontend/src/features/agents/AgentsView.vue`
- `frontend/src/styles/main.css`
- `frontend/e2e/devcenter.spec.ts`
