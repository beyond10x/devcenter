<script setup lang="ts">
import { ArrowRight, Blocks, KeyRound, ShieldCheck } from "@lucide/vue";
import { useWorkspaceStore } from "@/stores/workspace";

const workspace = useWorkspaceStore();

function providerLogin(id: string) {
  return `/auth/sso/start?provider=${encodeURIComponent(id)}`;
}
</script>

<template>
  <main class="landing">
    <header class="landing-header">
      <a href="/" class="brand-lockup" aria-label="Devcenter home">
        <span class="brand-glyph" aria-hidden="true">D</span>
        <span>Devcenter</span>
      </a>
      <a class="text-link" href="/docs">Documentation <ArrowRight :size="15" /></a>
    </header>

    <section class="landing-grid">
      <div class="hero-copy">
        <div class="eyebrow"><span class="live-dot"></span> Governed agent operations</div>
        <h1>Direct the work.<br /><span>Keep authority visible.</span></h1>
        <p>
          Create engineering agents, connect model access, and follow every task through one
          authenticated control surface.
        </p>
        <div class="hero-actions">
          <template v-if="workspace.identityProviders.length > 1">
            <span class="provider-prompt">Choose your verified identity provider</span>
            <a
              v-for="provider in workspace.identityProviders"
              :key="provider.id"
              class="button primary large"
              :href="providerLogin(provider.id)"
            >
              Continue with {{ provider.display_name }} <ArrowRight :size="17" />
            </a>
          </template>
          <a v-else class="button primary large" href="/auth/sso/start">
            Sign in through
            {{ workspace.identityProviders[0]?.display_name ?? "Identity" }}
            <ArrowRight :size="17" />
          </a>
          <span>Secure browser session · tenant and actor resolved server-side</span>
        </div>
      </div>

      <aside class="authority-panel" aria-label="Authority path">
        <div class="panel-heading">
          <span>Authority path</span>
          <span class="status-pill neutral">Fail closed</span>
        </div>
        <ol>
          <li>
            <span class="step-icon"><ShieldCheck :size="19" /></span>
            <div>
              <strong>Identity verifies you</strong><small>One opaque, HttpOnly session</small>
            </div>
            <span class="step-number">01</span>
          </li>
          <li>
            <span class="step-icon"><KeyRound :size="19" /></span>
            <div>
              <strong>Connectors holds access</strong
              ><small>Credential bytes stay out of Devcenter</small>
            </div>
            <span class="step-number">02</span>
          </li>
          <li>
            <span class="step-icon"><Blocks :size="19" /></span>
            <div>
              <strong>Attempts receive a lease</strong
              ><small>Bounded authority at execution time</small>
            </div>
            <span class="step-number">03</span>
          </li>
        </ol>
        <div class="panel-foot"><span class="pulse-dot"></span> Explicit BFF routes only</div>
      </aside>
    </section>

    <footer class="landing-footer">
      <span>Devcenter</span><span>Agents · Connections · Governed execution</span>
    </footer>
  </main>
</template>
