import { flushPromises, mount, type VueWrapper } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { HttpResponse, http } from "msw";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ConnectorProviderDescription } from "@/api/client";
import ConnectionsView from "@/features/connections/ConnectionsView.vue";
import { server } from "./setup";

vi.mock("vue-router", () => ({
  useRoute: () => ({ query: {} }),
}));

const connections = [
  {
    connection_ref: "connection:gitlab:user",
    integration_ref: "gitlab",
    label: "Work GitLab",
    state: "degraded",
    scope: "principal",
    actor: "user",
    auth_profile: "gitlab.oauth",
  },
  {
    connection_ref: "connection:slack:user",
    integration_ref: "slack",
    label: "Work Slack",
    state: "callable",
    scope: "principal",
    actor: "user",
    auth_profile: "slack.oauth",
  },
];

function providerDescription(providerRef: string): ConnectorProviderDescription {
  const oauthProfiles =
    providerRef === "grafana"
      ? []
      : [{ auth_profile: `${providerRef}.oauth`, actor: "person" as const }];
  return {
    provider: {
      provider_ref: providerRef,
      vendor: providerRef.charAt(0).toUpperCase() + providerRef.slice(1),
      description: `${providerRef} provider`,
      audiences: [],
      services: [],
      operation_count: 0,
      configurable: oauthProfiles.length > 0,
      setup_profiles: oauthProfiles,
    },
    operations: [],
  };
}

function useProviderHandlers(onStart?: (body: unknown) => void) {
  server.use(
    http.get("/api/connections", () => HttpResponse.json(connections)),
    http.get("/api/connectors/catalog/:providerRef", ({ params }) =>
      HttpResponse.json(providerDescription(String(params.providerRef))),
    ),
    http.post("/api/connections", async ({ request }) => {
      onStart?.(await request.json());
      return HttpResponse.json(
        {
          connect_session_ref: "connect-session:replacement",
          integration_ref: "gitlab",
          state: "failed",
          expires_at_unix_ms: 1_788_260_900_000,
        },
        { status: 201 },
      );
    }),
  );
}

function curatedCard(wrapper: VueWrapper, provider: string) {
  const card = wrapper
    .findAll(".curated-grid .provider-connection-card")
    .find((candidate) => candidate.text().includes(provider));
  if (!card) throw new Error(`${provider} curated card was not rendered`);
  return card;
}

describe("curated connection recovery", () => {
  beforeEach(() => {
    const pinia = createPinia();
    setActivePinia(pinia);
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it("shows callability, recovery actions, and deployment setup requirements", async () => {
    useProviderHandlers();
    const wrapper = mount(ConnectionsView, {
      props: { embedded: true },
      global: { plugins: [createPinia()] },
    });

    await flushPromises();

    const gitlab = curatedCard(wrapper, "GitLab");
    expect(gitlab.text()).toContain("Needs attention");
    expect(gitlab.get("button").text()).toContain("Reconnect GitLab");

    const slack = curatedCard(wrapper, "Slack");
    expect(slack.text()).toContain("Callable");
    expect(slack.get("button").text()).toContain("Replace authorization");

    const grafana = curatedCard(wrapper, "Grafana");
    expect(grafana.text()).toContain("Deployment administrator setup required");
    expect(grafana.find("button").exists()).toBe(false);
  });

  it("starts recovery with the existing label and provider-declared OAuth profile", async () => {
    let requestBody: unknown;
    useProviderHandlers((body) => {
      requestBody = body;
    });
    const wrapper = mount(ConnectionsView, {
      props: { embedded: true },
      global: { plugins: [createPinia()] },
    });
    await flushPromises();

    await curatedCard(wrapper, "GitLab").get("button").trigger("click");
    await flushPromises();

    expect(curatedCard(wrapper, "GitLab").text()).toContain("Authorization failed");
    expect(requestBody).toEqual({
      integration_ref: "gitlab",
      label: "Work GitLab",
      auth_profile: "gitlab.oauth",
    });
  });

  it("does not present application authority as a replaceable personal connection", async () => {
    let requestBody: unknown;
    server.use(
      http.get("/api/connections", () =>
        HttpResponse.json([
          {
            connection_ref: "connection:gitlab:application",
            integration_ref: "gitlab",
            label: "Deployment GitLab",
            state: "callable",
            initiation: ["b10x"],
            route: {},
            scope: "tenant",
            actor: "app",
            auth_profile: "gitlab.application",
          },
        ]),
      ),
      http.get("/api/connectors/catalog/:providerRef", ({ params }) => {
        const providerRef = String(params.providerRef);
        const description = providerDescription(providerRef);
        if (providerRef === "gitlab") {
          description.provider.setup_profiles = [
            { auth_profile: "gitlab.application", actor: "application" },
            { auth_profile: "gitlab.oauth", actor: "person" },
          ];
          description.provider.configurable = true;
        }
        return HttpResponse.json(description);
      }),
      http.post("/api/connections", async ({ request }) => {
        requestBody = await request.json();
        return HttpResponse.json(
          {
            connect_session_ref: "connect-session:person",
            integration_ref: "gitlab",
            state: "failed",
            expires_at_unix_ms: 1_788_260_900_000,
          },
          { status: 201 },
        );
      }),
    );
    const wrapper = mount(ConnectionsView, {
      props: { embedded: true },
      global: { plugins: [createPinia()] },
    });
    await flushPromises();

    const gitlab = curatedCard(wrapper, "GitLab");
    expect(gitlab.text()).toContain("Not connected");
    expect(gitlab.get("button").text()).toContain("Connect GitLab");

    await gitlab.get("button").trigger("click");
    await flushPromises();
    expect(requestBody).toEqual({
      integration_ref: "gitlab",
      label: "My GitLab",
      auth_profile: "gitlab.oauth",
    });
  });

  it("allows another recovery attempt when session status becomes unavailable", async () => {
    vi.useFakeTimers();
    useProviderHandlers();
    server.use(
      http.post("/api/connections", () =>
        HttpResponse.json(
          {
            connect_session_ref: "connect-session:pending",
            integration_ref: "gitlab",
            state: "pending",
            expires_at_unix_ms: 1_788_260_900_000,
          },
          { status: 201 },
        ),
      ),
      http.get("/api/connect-sessions/:sessionRef", () =>
        HttpResponse.json({ code: "connectors_unavailable" }, { status: 503 }),
      ),
    );
    const wrapper = mount(ConnectionsView, {
      props: { embedded: true },
      global: { plugins: [createPinia()] },
    });
    await flushPromises();

    await curatedCard(wrapper, "GitLab").get("button").trigger("click");
    await flushPromises();
    await vi.advanceTimersByTimeAsync(2_000);
    await flushPromises();

    const gitlab = curatedCard(wrapper, "GitLab");
    expect(gitlab.get("button").attributes("disabled")).toBeUndefined();
    expect(gitlab.get("button").text()).toContain("Reconnect GitLab");
  });

  it("fails an exhausted pending session after the bounded polling window", async () => {
    vi.useFakeTimers();
    let statusRequests = 0;
    useProviderHandlers();
    server.use(
      http.post("/api/connections", () =>
        HttpResponse.json(
          {
            connect_session_ref: "connect-session:pending",
            integration_ref: "gitlab",
            state: "pending",
            expires_at_unix_ms: 1_788_260_900_000,
          },
          { status: 201 },
        ),
      ),
      http.get("/api/connect-sessions/:sessionRef", () => {
        statusRequests += 1;
        return HttpResponse.json({
          connect_session_ref: "connect-session:pending",
          integration_ref: "gitlab",
          state: "pending",
          expires_at_unix_ms: 1_788_260_900_000,
        });
      }),
    );
    const wrapper = mount(ConnectionsView, {
      props: { embedded: true },
      global: { plugins: [createPinia()] },
    });
    await flushPromises();

    await curatedCard(wrapper, "GitLab").get("button").trigger("click");
    await flushPromises();
    for (let attempt = 0; attempt < 60; attempt += 1) {
      await vi.advanceTimersByTimeAsync(2_000);
      await flushPromises();
    }

    const gitlab = curatedCard(wrapper, "GitLab");
    expect(statusRequests).toBe(60);
    expect(gitlab.text()).toContain("Authorization failed");
    expect(gitlab.get("button").attributes("disabled")).toBeUndefined();
  });

  it("does not fail another provider's pending session when one status request fails", async () => {
    vi.useFakeTimers();
    useProviderHandlers();
    server.use(
      http.post("/api/connections", async ({ request }) => {
        const body = (await request.json()) as { integration_ref: string };
        return HttpResponse.json(
          {
            connect_session_ref: `connect-session:${body.integration_ref}`,
            integration_ref: body.integration_ref,
            state: "pending",
            expires_at_unix_ms: 1_788_260_900_000,
          },
          { status: 201 },
        );
      }),
      http.get("/api/connect-sessions/:sessionRef", ({ params }) => {
        const sessionRef = String(params.sessionRef);
        if (sessionRef.endsWith("gitlab")) {
          return HttpResponse.json({ code: "connectors_unavailable" }, { status: 503 });
        }
        return HttpResponse.json({
          connect_session_ref: sessionRef,
          integration_ref: "slack",
          state: "pending",
          expires_at_unix_ms: 1_788_260_900_000,
        });
      }),
    );
    const wrapper = mount(ConnectionsView, {
      props: { embedded: true },
      global: { plugins: [createPinia()] },
    });
    await flushPromises();

    await curatedCard(wrapper, "GitLab").get("button").trigger("click");
    await curatedCard(wrapper, "Slack").get("button").trigger("click");
    await flushPromises();
    await vi.advanceTimersByTimeAsync(2_000);
    await flushPromises();

    expect(curatedCard(wrapper, "GitLab").get("button").attributes("disabled")).toBeUndefined();
    expect(curatedCard(wrapper, "Slack").get("button").attributes("disabled")).toBeDefined();
  });

  it("keeps each provider disabled until its own start request finishes", async () => {
    useProviderHandlers();
    let requestCount = 0;
    const starts = new Map<string, () => void>();
    server.use(
      http.post("/api/connections", async () => {
        const integrationRef = requestCount === 0 ? "gitlab" : "slack";
        requestCount += 1;
        await new Promise<void>((resolve) => {
          starts.set(integrationRef, resolve);
        });
        return HttpResponse.json(
          {
            connect_session_ref: `connect-session:${integrationRef}`,
            integration_ref: integrationRef,
            state: "failed",
            expires_at_unix_ms: 1_788_260_900_000,
          },
          { status: 201 },
        );
      }),
    );
    const wrapper = mount(ConnectionsView, {
      props: { embedded: true },
      global: { plugins: [createPinia()] },
    });
    await flushPromises();

    await curatedCard(wrapper, "GitLab").get("button").trigger("click");
    await flushPromises();
    await curatedCard(wrapper, "Slack").get("button").trigger("click");
    await flushPromises();

    const resolveGitLab = starts.get("gitlab");
    const resolveSlack = starts.get("slack");
    if (!resolveGitLab || !resolveSlack) throw new Error("both start requests were not observed");
    resolveGitLab();
    await flushPromises();
    const slackDisabledWhileStarting = curatedCard(wrapper, "Slack")
      .get("button")
      .attributes("disabled");
    resolveSlack();
    await flushPromises();

    expect(slackDisabledWhileStarting).toBeDefined();
  });

  it("keeps the browser completion URL available when the authorization popup is blocked", async () => {
    vi.useFakeTimers();
    const open = vi.spyOn(window, "open").mockReturnValue(null);
    useProviderHandlers();
    server.use(
      http.post("/api/connections", () =>
        HttpResponse.json(
          {
            connect_session_ref: "connect-session:pending",
            integration_ref: "gitlab",
            state: "pending",
            expires_at_unix_ms: 1_788_260_900_000,
            browser_completion_url: "https://connectors.example/connect-sessions/pending",
          },
          { status: 201 },
        ),
      ),
    );
    const wrapper = mount(ConnectionsView, {
      props: { embedded: true },
      global: { plugins: [createPinia()] },
    });
    await flushPromises();

    await curatedCard(wrapper, "GitLab").get("button").trigger("click");
    await flushPromises();

    const gitlab = curatedCard(wrapper, "GitLab");
    expect(open).toHaveBeenCalledWith(
      "https://connectors.example/connect-sessions/pending",
      "_blank",
      "noopener,noreferrer",
    );
    expect(
      gitlab.find('a[href="https://connectors.example/connect-sessions/pending"]').exists(),
    ).toBe(true);
  });

  it("clears a failed recovery message after refresh observes callable authority", async () => {
    let connectionState = "degraded";
    useProviderHandlers();
    server.use(
      http.get("/api/connections", () =>
        HttpResponse.json(
          connections.map((connection) =>
            connection.integration_ref === "gitlab"
              ? { ...connection, state: connectionState }
              : connection,
          ),
        ),
      ),
    );
    const wrapper = mount(ConnectionsView, {
      props: { embedded: true },
      global: { plugins: [createPinia()] },
    });
    await flushPromises();

    await curatedCard(wrapper, "GitLab").get("button").trigger("click");
    await flushPromises();
    expect(curatedCard(wrapper, "GitLab").text()).toContain("Authorization failed");

    connectionState = "callable";
    const refresh = wrapper.findAll("button").find((button) => button.text().trim() === "Refresh");
    if (!refresh) throw new Error("provider refresh button was not rendered");
    await refresh.trigger("click");
    await flushPromises();

    const gitlab = curatedCard(wrapper, "GitLab");
    expect(gitlab.text()).toContain("Callable");
    expect(gitlab.text()).not.toContain("Authorization failed");
  });
});
