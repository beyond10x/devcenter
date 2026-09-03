import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import RenderedMarkdown from "@/components/RenderedMarkdown.vue";

describe("RenderedMarkdown", () => {
  it("renders agent Markdown and updates incrementally", async () => {
    const wrapper = mount(RenderedMarkdown, {
      props: { source: "## Result\n\nFirst" },
    });

    expect(wrapper.find("h2").text()).toBe("Result");
    await wrapper.setProps({ source: "## Result\n\nFirst **complete**" });
    expect(wrapper.find("strong").text()).toBe("complete");
  });

  it("does not admit model-supplied HTML or unsafe links", () => {
    const wrapper = mount(RenderedMarkdown, {
      props: {
        source: '<script>alert("no")</script> [unsafe](javascript:alert("no"))',
      },
    });

    expect(wrapper.find("script").exists()).toBe(false);
    expect(wrapper.find('a[href^="javascript:"]').exists()).toBe(false);
    expect(wrapper.text()).toContain("<script>");
  });
});
