import { effectScope } from "vue";
import { afterEach, expect, it, vi } from "vitest";
import { api, type ConnectSession } from "@/api/client";
import { useConnectSessionPolling } from "@/composables/useConnectSessionPolling";

function pending(ref: string): ConnectSession {
  return {
    connect_session_ref: ref,
    integration_ref: "gitlab",
    state: "pending",
    expires_at_unix_ms: Date.now() + 300_000,
  };
}

afterEach(() => {
  vi.restoreAllMocks();
  vi.useRealTimers();
});

it("ignores an old in-flight response after a replacement session starts", async () => {
  vi.useFakeTimers();
  let resolveOld!: (session: ConnectSession) => void;
  vi.spyOn(api, "connectionSession").mockImplementation(
    () =>
      new Promise((resolve) => {
        resolveOld = resolve;
      }),
  );
  let session = pending("old");
  const completed = vi.fn(async () => {});
  const scope = effectScope();
  const poller = scope.run(() =>
    useConnectSessionPolling(
      () => session,
      (_key, value) => {
        session = value;
      },
      completed,
    ),
  );
  if (!poller) throw new Error("Polling scope did not start");
  poller.start("provider");
  await vi.advanceTimersByTimeAsync(2_000);
  session = pending("new");
  poller.start("provider");
  resolveOld({ ...pending("old"), state: "completed", connection_ref: "old-connection" });
  await vi.advanceTimersByTimeAsync(0);
  expect(session.connect_session_ref).toBe("new");
  expect(session.state).toBe("pending");
  expect(completed).not.toHaveBeenCalled();
  scope.stop();
});

it("cancels scheduled work and ignores responses after the view is disposed", async () => {
  vi.useFakeTimers();
  let resolve!: (session: ConnectSession) => void;
  const read = vi.spyOn(api, "connectionSession").mockImplementation(
    () =>
      new Promise((done) => {
        resolve = done;
      }),
  );
  const session = pending("current"),
    update = vi.fn(),
    completed = vi.fn(async () => {});
  const scope = effectScope();
  const poller = scope.run(() => useConnectSessionPolling(() => session, update, completed));
  if (!poller) throw new Error("Polling scope did not start");
  poller.start("first");
  await vi.advanceTimersByTimeAsync(2_000);
  poller.start("second");
  scope.stop();
  resolve({ ...session, state: "completed", connection_ref: "saved" });
  await vi.advanceTimersByTimeAsync(10_000);
  expect(read).toHaveBeenCalledTimes(1);
  expect(update).not.toHaveBeenCalled();
  expect(completed).not.toHaveBeenCalled();
});
