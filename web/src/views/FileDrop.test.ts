import { describe, it, expect, vi } from "vitest";
import { FileDrop } from "./FileDrop";

function setup() {
  const handlers: Record<string, Function> = {};
  const dropZone = {
    addEventListener: vi.fn((event: string, handler: any) => {
      handlers[event] = handler;
    }),
    removeEventListener: vi.fn(),
    classList: {
      add: vi.fn(),
      remove: vi.fn(),
    },
  } as unknown as HTMLElement;

  const fileInput = {
    addEventListener: vi.fn((event: string, handler: any) => {
      handlers[event] = handler;
    }),
    removeEventListener: vi.fn(),
    files: null as FileList | null,
  } as unknown as HTMLInputElement;

  return { handlers, dropZone, fileInput };
}

describe("FileDrop", () => {
  it("drop event calls onFile callback with the dropped File", () => {
    const { handlers, dropZone, fileInput } = setup();
    const fd = new FileDrop(dropZone, fileInput);
    const onFile = vi.fn();
    fd.onFile(onFile);

    const fakeFile = new File(["test"], "model.stl", { type: "application/octet-stream" });
    const dropEvent = { preventDefault: vi.fn(), dataTransfer: { files: [fakeFile] } };

    handlers.drop(dropEvent);

    expect(onFile).toHaveBeenCalledWith(fakeFile);
  });

  it("dragover event adds drag-over class and prevents default", () => {
    const { handlers, dropZone, fileInput } = setup();
    new FileDrop(dropZone, fileInput);

    const preventDefault = vi.fn();
    handlers.dragover({ preventDefault });

    expect(preventDefault).toHaveBeenCalled();
    expect(dropZone.classList.add).toHaveBeenCalledWith("drag-over");
  });

  it("dragleave event removes drag-over class", () => {
    const { handlers, dropZone, fileInput } = setup();
    new FileDrop(dropZone, fileInput);

    handlers.dragleave();
    expect(dropZone.classList.remove).toHaveBeenCalledWith("drag-over");
  });

  it("dispose removes all listeners", () => {
    const { dropZone, fileInput } = setup();
    const fd = new FileDrop(dropZone, fileInput);
    fd.dispose();

    // removeEventListener should be called for each registered listener
    expect(dropZone.removeEventListener).toHaveBeenCalled();
    expect(fileInput.removeEventListener).toHaveBeenCalled();
  });
});
