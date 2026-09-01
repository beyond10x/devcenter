import { describe, expect, it } from "vitest";
import { ApiError, errorMessage } from "@/api/client";

describe("API errors", () => {
  it("explains the server's canonical expired-session code", () => {
    expect(errorMessage(new ApiError(401, "identity_authentication_required"))).toBe(
      "Your session has expired. Sign in again.",
    );
  });
});
