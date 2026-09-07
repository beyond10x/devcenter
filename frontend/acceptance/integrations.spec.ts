import { test, expect } from "@playwright/test";
import { createHash, randomUUID } from "node:crypto";
import { writeFileSync } from "node:fs";
import { z } from "zod";

const live = process.env.DEVCENTER_CONNECTOR_MODE === "live";
const providers = z
  .array(z.string().regex(/^[a-z0-9][a-z0-9_-]*$/))
  .parse(JSON.parse(process.env.DEVCENTER_REQUIRED_INTEGRATIONS ?? "[]"));
const connectionSchema = z.object({ connection_ref: z.string(), provider: z.string() });
const operationSchema = z.object({
  operation_ref: z.string(),
  effect: z.string(),
  approval: z.string(),
  connections: z.array(connectionSchema),
});

test("live integration targets are declared by the composed deployment", () => {
  test.skip(!live, "Fixture mode does not establish real integration access");
  expect(
    providers.length,
    "No real integrations were selected from the deployment",
  ).toBeGreaterThan(0);
});

for (const provider of providers) {
  test(`real ${provider} admits and completes a provider read`, async ({ context }) => {
    test.skip(!live, "Fixture mode does not establish real integration access");
    process.umask(0o077);
    const origin = z.url().parse(process.env.DEVCENTER_ORIGIN);
    const root = z.string().parse(process.env.DEVCENTER_EVIDENCE_ROOT);
    const evidence: Record<string, unknown> = {
      connector_mode: "live",
      provider,
      result: "NOT_COMPLETED",
    };
    try {
      const sessionResponse = await context.request.get(`${origin}/api/session`);
      expect(sessionResponse.status()).toBe(200);
      const session = z.object({ tenant_id: z.string() }).parse(await sessionResponse.json());
      const cookie = (await context.cookies(origin)).find((entry) =>
        entry.value.startsWith("identity_session_v1_"),
      );
      if (!cookie) throw new Error("Identity-issued browser session missing");
      const tokenResponse = await context.request.post(`${origin}/v1/access-token`, {
        headers: { Authorization: `Bearer ${cookie.value}`, Origin: origin },
        data: {
          audience: "urn:b10x:connectors",
          scope: "connectors.catalog.read connectors.invoke",
        },
      });
      expect(tokenResponse.status()).toBe(200);
      const access = z.object({ access_token: z.string() }).parse(await tokenResponse.json());
      // Match the BFF's session-bound correlation context. Tenant comes from authenticated
      // Identity, and Connectors independently revalidates Identity and current grants.
      const owner = {
        tenant_id: session.tenant_id,
        agent_id: "devcenter-browser",
        agent_revision: 1,
        authority_snapshot_id: "devcenter-session",
        authority_snapshot_sha256: createHash("sha256").update(cookie.value).digest("hex"),
      };
      async function operation(method: string, params: Record<string, unknown>) {
        const requestId = randomUUID();
        const response = await context.request.post(`${origin}/api/connectors/v1/operations`, {
          headers: { Authorization: `Bearer ${access.access_token}`, Origin: origin },
          data: {
            protocol: "b10x.connector-operation.v0alpha3",
            request_id: requestId,
            context: owner,
            request: { method, params },
          },
        });
        evidence[`${method}_status`] = response.status();
        const envelope = z
          .object({
            request_id: z.string(),
            response: z.object({ result: z.string(), value: z.unknown() }).nullish(),
            error: z.object({ code: z.string() }).nullish(),
          })
          .parse(await response.json());
        expect(envelope.request_id).toBe(requestId);
        if (!response.ok() || envelope.error || envelope.response?.result !== method) {
          evidence.failure_code = envelope.error?.code ?? "invalid_response";
          throw new Error(`${provider} ${method} refused: ${String(evidence.failure_code)}`);
        }
        return envelope.response.value;
      }
      const search = z
        .object({ operations: z.array(operationSchema) })
        .parse(await operation("search", { query: provider, limit: 25 }));
      const candidates = search.operations.filter(
        (item) =>
          item.effect === "read_only" &&
          item.approval === "not_required" &&
          item.connections.some((connection) => connection.provider === provider),
      );
      if (!candidates.length) {
        evidence.result = "AUTHORIZATION_REQUIRED";
        throw new Error(`Connect ${provider} at ${origin}/connectors/${provider}, then retry.`);
      }
      for (const candidate of candidates) {
        const description = operationSchema
          .extend({
            description_ref: z.string(),
            input_schema: z.record(z.string(), z.unknown()),
          })
          .parse(await operation("describe", { operation_ref: candidate.operation_ref }));
        const required = description.input_schema.required;
        if (
          description.input_schema.type !== "object" ||
          (required !== undefined && (!Array.isArray(required) || required.length > 0))
        )
          continue;
        expect(description.operation_ref).toBe(candidate.operation_ref);
        expect(description.effect).toBe("read_only");
        expect(description.approval).toBe("not_required");
        const connection = description.connections.find((item) => item.provider === provider);
        if (!connection) continue;
        evidence.operation_ref = description.operation_ref;
        evidence.connection_ref = connection.connection_ref;
        const invoked = z
          .object({ operation_ref: z.string(), connector_audit_ref: z.string().min(1) })
          .parse(
            await operation("invoke", {
              operation_ref: description.operation_ref,
              connection_ref: connection.connection_ref,
              description_ref: description.description_ref,
              input: {},
            }),
          );
        expect(invoked.operation_ref).toBe(description.operation_ref);
        evidence.connector_audit_ref = invoked.connector_audit_ref;
        evidence.result = "LIVE_PROVIDER_READ_PASS";
        return;
      }
      throw new Error(`${provider} has no admitted read accepting an empty input object`);
    } finally {
      // Store statuses and audit references, never provider response bodies or credentials.
      writeFileSync(`${root}/integration-${provider}.json`, JSON.stringify(evidence, null, 2));
    }
  });
}
