import { onScopeDispose, ref } from "vue";
import { api, errorMessage, type ConnectSession } from "@/api/client";

/** Poll only the issued session; a failed status read never restarts authorization. */
export function useConnectSessionPolling(
  current: (key: string) => ConnectSession | undefined,
  update: (key: string, session: ConnectSession) => void,
  completed: () => Promise<void>,
) {
  const errors = ref<Record<string, string>>({});
  const checking = ref<Record<string, boolean>>({});
  const timers = new Map<string, ReturnType<typeof setTimeout>>();
  const generations = new Map<string, number>();
  let disposed = false;

  function clearError(key: string) {
    errors.value = Object.fromEntries(
      Object.entries(errors.value).filter(([entry]) => entry !== key),
    );
  }

  function cancel(key: string) {
    clearTimeout(timers.get(key));
    timers.delete(key);
    const generation = (generations.get(key) ?? 0) + 1;
    generations.set(key, generation);
    return generation;
  }

  function schedule(key: string, generation: number) {
    timers.set(
      key,
      setTimeout(() => void poll(key, generation), 2_000),
    );
  }

  async function poll(key: string, generation: number) {
    timers.delete(key);
    const session = current(key);
    if (!session || disposed || generations.get(key) !== generation) return;
    const isCurrent = () =>
      !disposed &&
      generations.get(key) === generation &&
      current(key)?.connect_session_ref === session.connect_session_ref;
    checking.value[key] = true;
    try {
      const next = await api.connectionSession(session.connect_session_ref);
      if (!isCurrent()) return;
      update(key, next);
      clearError(key);
      if (next.state === "completed") await completed();
      if (next.state === "pending") {
        if (Date.now() >= next.expires_at_unix_ms) {
          errors.value[key] = "The authorization deadline passed. Check its final status.";
        } else {
          schedule(key, generation);
        }
      }
    } catch (cause) {
      if (isCurrent()) errors.value[key] = errorMessage(cause);
    } finally {
      if (isCurrent()) checking.value[key] = false;
    }
  }

  function start(key: string) {
    const generation = cancel(key);
    clearError(key);
    checking.value[key] = false;
    if (current(key)?.state === "pending") schedule(key, generation);
  }

  function retry(key: string) {
    const generation = cancel(key);
    clearError(key);
    void poll(key, generation);
  }

  onScopeDispose(() => {
    disposed = true;
    for (const timer of timers.values()) clearTimeout(timer);
    timers.clear();
  });

  return { errors, checking, start, retry };
}
