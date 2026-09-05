<!--
  The softphone, mounted from `@b10x/phone-widget`.

  Everything about a call lives in that package: the specification's own wasm system, the control
  channel to `phone-server`, and the `RTCPeerConnection`. This view supplies the two deployment
  facts and nothing else, which is what keeps them out of this repository — the WebSocket address is
  read from the environment at build time and falls back to a loopback default, and the module's
  bytes are resolved as an asset by Vite.
-->
<script setup lang="ts">
import { PhonePanel } from "@b10x/phone-widget";
import wasmUrl from "@b10x/phone-widget/softphone.wasm?url";
import "@b10x/phone-widget/style.css";

// A `phone-server` runs beside the engineer, not in the cluster, and the widget is told where by
// configuration rather than by a constant in source. The default is the address that binary listens
// on out of the box.
const endpoint = import.meta.env.VITE_PHONE_ENDPOINT ?? "ws://127.0.0.1:8780";
</script>

<template>
  <section class="p-6">
    <h1 class="text-lg font-semibold">Phone</h1>
    <p class="mt-1 max-w-prose text-sm opacity-70">
      Calls out through a <code>phone-server</code> running beside you. It terminates the browser's
      audio, places the SIP call and forwards between the two, so nothing here holds a SIP stack.
    </p>
    <div class="mt-6">
      <PhonePanel :wasm="wasmUrl" :endpoint="endpoint" label="Devcenter" />
    </div>
  </section>
</template>
