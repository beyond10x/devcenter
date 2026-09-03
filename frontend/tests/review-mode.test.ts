import { expect, test } from "vitest";
import { createServer } from "vite";
import { server as mockServer } from "./setup";

test("review mode serves governed consoles and the automatic hosted workbench", async () => {
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

    const sessionId = "workspace-session-review";
    const resumedResponse = await fetch(`${origin}/api/project-sessions/${sessionId}/resume`, {
      method: "POST",
    });
    expect(resumedResponse.status).toBe(200);
    expect(await resumedResponse.json()).toMatchObject({
      id: sessionId,
      coordination: { state: "ready", retryable: false },
    });

    const coordinationResponse = await fetch(
      `${origin}/api/project-sessions/${sessionId}/coordination`,
    );
    expect(coordinationResponse.status).toBe(200);
    expect(await coordinationResponse.json()).toMatchObject({
      summary: { state: "ready" },
      session: { session_id: sessionId, workspace_session_id: sessionId, state: "Active" },
    });

    const grantResponse = await fetch(
      `${origin}/api/project-sessions/${sessionId}/coordination/grants`,
      {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ grantee: "agent-review", idempotency_key: "review-grant" }),
      },
    );
    expect(grantResponse.status).toBe(200);
    expect(await grantResponse.json()).toMatchObject({
      grants: [
        {
          grantee: "agent-review",
          allowed_intents: ["code_edit", "code_create", "code_delete", "code_rename"],
          state: "Active",
        },
      ],
    });

    const terminalResponse = await fetch(`${origin}/api/project-sessions/${sessionId}/terminals`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({
        profile_id: "rust-stable-confined",
        columns: 100,
        rows: 28,
        idempotency_key: "review-terminal",
      }),
    });
    expect(terminalResponse.status).toBe(201);
    expect(await terminalResponse.json()).toMatchObject({
      coding_session_id: sessionId,
      agentide_session_id: sessionId,
      state: "running",
    });
    const terminalCoordinationResponse = await fetch(
      `${origin}/api/project-sessions/${sessionId}/coordination`,
    );
    const terminalCoordination = (await terminalCoordinationResponse.json()) as {
      grants: Array<{ grantee: string; allowed_intents: string[]; state: string }>;
    };
    expect(
      terminalCoordination.grants.find((grant) =>
        grant.allowed_intents.includes("interactive_terminal"),
      ),
    ).toMatchObject({
      grantee: "review-engineer",
      allowed_intents: ["interactive_terminal"],
      state: "Active",
    });

    const turnResponse = await fetch(
      `${origin}/api/project-sessions/${sessionId}/agents/agent-review/turns`,
      {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ prompt: "Review the workspace." }),
      },
    );
    expect(turnResponse.status).toBe(202);
    expect(await turnResponse.json()).toMatchObject({
      agent_id: "agent-review",
      workspace_session_id: sessionId,
      agentide_session_id: sessionId,
    });
  } finally {
    await server.close();
    mockServer.listen({ onUnhandledRequest: "error" });
  }
});
