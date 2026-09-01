import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import SignInView from "@/features/session/SignInView.vue";
import { createPinia, setActivePinia } from "pinia";
import { beforeEach } from "vitest";
import { useWorkspaceStore } from "@/stores/workspace";

describe("signed-out experience", () => {
  beforeEach(() => setActivePinia(createPinia()));

  it("starts generic Identity login and explains the authority boundary", () => {
    const wrapper = mount(SignInView);
    expect(wrapper.get('a[href="/auth/sso/start"]').text()).toContain("Sign in through Identity");
    expect(wrapper.text()).toContain("Identity verifies you");
    expect(wrapper.text()).toContain("Credential bytes stay out of Devcenter");
    expect(wrapper.text()).toContain("Attempts receive a lease");
  });

  it("renders configured providers with opaque provider IDs", () => {
    const pinia = createPinia();
    setActivePinia(pinia);
    const workspace = useWorkspaceStore();
    workspace.identityProviders = [
      { id: "provider_one", display_name: "Provider One" },
      { id: "provider-two", display_name: "Provider Two" },
    ];
    const wrapper = mount(SignInView, { global: { plugins: [pinia] } });
    const links = wrapper.findAll(".hero-actions a");
    expect(links.map((link) => link.attributes("href"))).toEqual([
      "/auth/sso/start?provider=provider_one",
      "/auth/sso/start?provider=provider-two",
    ]);
  });
});
