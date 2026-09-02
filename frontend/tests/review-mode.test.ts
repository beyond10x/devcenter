import { expect, test } from "vitest";
import { createServer } from "vite";
import { server as mockServer } from "./setup";

test("review mode publishes and serves a new capability profile", async () => {
  mockServer.close();
  const server = await createServer({
    mode: "review",
    server: { host: "127.0.0.1", port: 0 },
  });
  await server.listen();

  try {
    const address = server.httpServer?.address();
    if (!address || typeof address === "string") throw new Error("review server did not bind TCP");
    const origin = `http://127.0.0.1:${String(address.port)}`;
    const createdResponse = await fetch(`${origin}/api/mcp/publications`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ profile_id: "profile-review-created" }),
    });
    expect(createdResponse.status).toBe(201);
    const created = (await createdResponse.json()) as {
      publication_id: string;
      profile_id: string;
      state: string;
    };
    expect(created).toMatchObject({ profile_id: "profile-review-created", state: "active" });

    const publicationsResponse = await fetch(`${origin}/api/mcp/publications`);
    const publications = (await publicationsResponse.json()) as Array<{
      publication_id: string;
    }>;
    expect(publications[0]?.publication_id).toBe(created.publication_id);

    const clientsResponse = await fetch(
      `${origin}/api/mcp/publications/${encodeURIComponent(created.publication_id)}/clients`,
    );
    expect(clientsResponse.status).toBe(200);
    expect(await clientsResponse.json()).toEqual([]);

    const capabilitiesResponse = await fetch(`${origin}/api/capabilities`);
    expect(capabilitiesResponse.status).toBe(200);
    expect(await capabilitiesResponse.json()).toHaveLength(3);

    const profilesResponse = await fetch(`${origin}/api/capability-profiles`);
    expect(profilesResponse.status).toBe(200);
    const profiles = (await profilesResponse.json()) as Array<{
      id: string;
      revision: number;
      mappings: unknown[];
    }>;
    expect(profiles[0]).toMatchObject({ id: "profile-release-operations", revision: 4 });
    expect(profiles[0]?.mappings).toHaveLength(3);
  } finally {
    await server.close();
    mockServer.listen({ onUnhandledRequest: "error" });
  }
});
