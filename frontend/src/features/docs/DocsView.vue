<script setup lang="ts">
import { ArrowUpRight, BookOpen, Braces, ShieldCheck } from "@lucide/vue";
</script>

<template>
  <div class="view docs-view">
    <header class="docs-hero">
      <div>
        <p class="eyebrow">Product documentation</p>
        <h1>Devcenter, by contract</h1>
        <p>
          The authenticated browser application and deliberately narrow BFF for independent
          identity, agent, connector, workflow, and execution services.
        </p>
      </div>
      <a class="button quiet" href="/openapi.json"
        ><Braces :size="17" /> OpenAPI 3.1 <ArrowUpRight :size="15"
      /></a>
    </header>
    <div class="docs-layout">
      <nav class="docs-nav" aria-label="Documentation sections">
        <p>On this page</p>
        <a href="#sign-in">Sign in</a><a href="#connect-claude-code">Connect Claude Code</a
        ><a href="#agents">Create and run an agent</a><a href="#api">API and MCP</a
        ><a href="#deployment">Deployment</a>
      </nav>
      <article class="docs-content">
        <section id="sign-in">
          <span class="section-icon"><ShieldCheck :size="20" /></span>
          <h2>Sign in</h2>
          <p>
            <code>GET /auth/sso/start</code> begins generic OpenID Connect login through the
            configured Identity service. Devcenter stores only an opaque, secure browser-session
            cookie. Provider branding and deployment-specific organization policy remain outside
            this repository.
          </p>
        </section>
        <section id="connect-claude-code">
          <span class="section-icon"><BookOpen :size="20" /></span>
          <h2>Connect Claude Code</h2>
          <p>
            After signing in, start <strong>Connect Claude</strong>, authorize the public client
            with Claude, and return the one-time code. Connectors owns the PKCE verifier, provider
            exchange, encrypted access and refresh-token custody, refresh, replacement, and
            revocation.
          </p>
          <div class="docs-callout">
            <strong>Attempt boundary</strong>
            <p>
              When a user starts a task, Agent Platform requests an exact attempt-bound lease.
              Harness redeems that lease only at the provider call.
            </p>
          </div>
        </section>
        <section id="agents">
          <h2>Create and run an agent</h2>
          <p>
            The Agents workspace creates an agent, writes its first revision, activates it, and
            accepts prompts as Tasks. Results arrive as ordered server-sent events from
            <code>/api/tasks/{task_id}/events</code>. Durable workers and recovery remain separate
            deployment milestones.
          </p>
        </section>
        <section id="api">
          <h2>API and MCP</h2>
          <p>
            The deterministic <a href="/openapi.json">OpenAPI document</a> describes the BFF
            allowlist. Identity supplies generic verified authority while each downstream service
            keeps its own authorization vocabulary. <code>/mcp</code> exposes governed composition;
            it is not an unrestricted inner-service proxy.
          </p>
        </section>
        <section id="deployment">
          <h2>Deployment</h2>
          <p>
            The public Helm chart is configuration-neutral. Tenant policy, hosts, immutable private
            images, storage, Secret references, and private connector bundles are supplied by a
            separate deployment repository.
          </p>
        </section>
      </article>
    </div>
  </div>
</template>
