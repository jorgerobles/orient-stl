import { describe, it, expect, vi } from "vitest";
import { ConfigPanel } from "./ConfigPanel";

function mockInput(overrides = {}): HTMLInputElement {
  return {
    value: "30",
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    checked: false,
    ...overrides,
  } as unknown as HTMLInputElement;
}

function mockSelect(overrides = {}): HTMLSelectElement {
  return {
    value: "resin-biased",
    innerHTML: "",
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    ...overrides,
  } as unknown as HTMLSelectElement;
}

function mockButton(overrides = {}): HTMLButtonElement {
  return {
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    disabled: false,
    ...overrides,
  } as unknown as HTMLButtonElement;
}

function mockSpan(overrides = {}): HTMLElement {
  return {
    textContent: "",
    ...overrides,
  } as unknown as HTMLElement;
}

describe("ConfigPanel", () => {
  it("angle slider change fires onChange callback", () => {
    const handlers: Record<string, Function> = {};
    const angleSlider = mockInput({
      addEventListener: vi.fn((event: string, handler: any) => {
        handlers[event] = handler;
      }),
    });
    const angleValue = mockSpan();
    const hullSphereToggle = mockInput();
    const conventionSelect = mockSelect();
    const profileSelect = mockSelect();
    const rankerSelect = mockSelect();
    const findBtn = mockButton();
    const exportBtn = mockButton();
    const recalcBtn = mockButton();

    const panel = new ConfigPanel(
      angleSlider, angleValue, hullSphereToggle,
      conventionSelect, profileSelect, rankerSelect,
      findBtn, exportBtn, recalcBtn,
    );

    const onChange = vi.fn();
    panel.onChange(onChange);

    // Simulate input event on angle slider
    handlers.input?.();

    expect(onChange).toHaveBeenCalled();
  });

  it("getProfile and setProfile round-trip correctly", () => {
    const panel = new ConfigPanel(
      mockInput(), mockSpan(), mockInput(),
      mockSelect(), mockSelect(), mockSelect(),
      mockButton(), mockButton(), mockButton(),
    );

    panel.setProfile("topsis");
    expect(panel.getProfile()).toBe("topsis");
  });

  it("getAngle and setAngle round-trip correctly", () => {
    const angleSlider = mockInput();
    const angleValue = mockSpan();
    const panel = new ConfigPanel(
      angleSlider, angleValue, mockInput(),
      mockSelect(), mockSelect(), mockSelect(),
      mockButton(), mockButton(), mockButton(),
    );

    panel.setAngle(45);
    expect(panel.getAngle()).toBe(45);
    expect(angleSlider.value).toBe("45");
    expect(angleValue.textContent).toBe("45");
  });
});
