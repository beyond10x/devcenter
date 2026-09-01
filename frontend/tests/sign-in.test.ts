import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import SignInView from "@/features/session/SignInView.vue";

describe("signed-out experience", () => {
  it("starts generic Identity login and explains the authority boundary", () => {
    const wrapper = mount(SignInView);
    expect(wrapper.get('a[href="/auth/sso/start"]').text()).toContain("Sign in through Identity");
    expect(wrapper.text()).toContain("Identity verifies you");
    expect(wrapper.text()).toContain("Credential bytes stay out of Devcenter");
    expect(wrapper.text()).toContain("Attempts receive a lease");
  });
});
