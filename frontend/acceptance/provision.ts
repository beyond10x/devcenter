import type { APIRequestContext } from "@playwright/test";
import { lstatSync, readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { randomUUID } from "node:crypto";
import { z } from "zod";

const statusSchema = z.object({
  integrations: z.array(
    z.object({
      integration_ref: z.string(),
      active: z.boolean(),
      credentials: z.array(z.object({ name: z.string(), state: z.string() })),
    }),
  ),
});
const planSchema = z.array(
  z
    .object({
      integration: z.string().min(1),
      credential: z.string().min(1),
      value_file: z.string().min(1),
    })
    .strict(),
);

function privateText(path: string): string {
  const info = lstatSync(path);
  if (!info.isFile() || (info.mode & 0o077) !== 0 || info.size > 8192)
    throw new Error("provisioning input must be a bounded owner-only regular file");
  return readFileSync(path, "utf8");
}

/** External administration through the normal provider-neutral Connector contract. */
export async function provisionCredentials(
  request: APIRequestContext,
  origin: string,
  headers: Record<string, string>,
) {
  const planPath = z.string().parse(process.env.DEVCENTER_PROVISIONING_FILE);
  const plan = planSchema.parse(JSON.parse(privateText(planPath)));
  const base = `${origin}/api/connectors/v1/admin/integrations`;
  async function discover() {
    const response = await request.get(base, { headers });
    if (!response.ok())
      throw new Error(`credential discovery refused: ${String(response.status())}`);
    return statusSchema.parse(await response.json());
  }
  const before = await discover();
  for (const item of plan) {
    const integration = before.integrations.find(
      (entry) => entry.integration_ref === item.integration && entry.active,
    );
    if (!integration?.credentials.some((entry) => entry.name === item.credential))
      throw new Error("requested credential is not declared by an active Integration");
    const response = await request.put(
      `${base}/${encodeURIComponent(item.integration)}/credentials/${encodeURIComponent(item.credential)}`,
      {
        headers,
        data: {
          request_id: randomUUID(),
          value: privateText(resolve(dirname(planPath), item.value_file)).trimEnd(),
          replace: true,
        },
      },
    );
    if (!response.ok())
      throw new Error(`credential provisioning refused: ${String(response.status())}`);
  }
  const after = await discover();
  for (const item of plan) {
    if (
      after.integrations
        .find((entry) => entry.integration_ref === item.integration)
        ?.credentials.find((entry) => entry.name === item.credential)?.state !== "present"
    )
      throw new Error("credential custody was not confirmed");
  }
}
