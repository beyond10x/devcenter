import { describe, expect, it } from "vitest";
import { ApiError, errorMessage } from "@/api/client";

describe("API errors", () => {
  it("explains the server's canonical expired-session code", () => {
    expect(errorMessage(new ApiError(401, "identity_authentication_required"))).toBe(
      "Your session has expired. Sign in again.",
    );
  });

  it("keeps connection authorization separate from Identity session expiry", () => {
    const error = new ApiError(409, "service_connection_authentication_required");
    expect(errorMessage(error)).toContain("Open Connections to reconnect");
    expect(errorMessage(error)).not.toContain("session has expired");
    expect(errorMessage(new ApiError(429, "service_operation_rate_limited"))).toContain(
      "rate limiting",
    );
  });

  it("preserves structured conflict details", () => {
    const details = { code: "workspace_file_conflict", latest: { revision: { sha256: "new" } } };
    const error = new ApiError(409, "workspace_file_conflict", details);
    expect(error.details).toBe(details);
    expect(errorMessage(error)).toContain("changed after it was loaded");
  });
});
