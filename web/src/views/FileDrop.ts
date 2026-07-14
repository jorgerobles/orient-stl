export class FileDrop {
  private onFileCallback: ((file: File) => void) | null = null;
  private boundDragover: (e: DragEvent) => void;
  private boundDragleave: () => void;
  private boundDrop: (e: DragEvent) => void;
  private boundChange: () => void;

  constructor(
    private dropZone: HTMLElement,
    private fileInput: HTMLInputElement,
  ) {
    this.boundDragover = (e: DragEvent) => {
      e.preventDefault();
      dropZone.classList.add('drag-over');
    };
    this.boundDragleave = () => {
      dropZone.classList.remove('drag-over');
    };
    this.boundDrop = (e: DragEvent) => {
      e.preventDefault();
      dropZone.classList.remove('drag-over');
      if (e.dataTransfer?.files && e.dataTransfer.files.length > 0) {
        this.onFileCallback?.(e.dataTransfer.files[0]);
      }
    };
    this.boundChange = () => {
      if (fileInput.files && fileInput.files.length > 0) {
        this.onFileCallback?.(fileInput.files[0]);
      }
    };

    dropZone.addEventListener('dragover', this.boundDragover);
    dropZone.addEventListener('dragleave', this.boundDragleave);
    dropZone.addEventListener('drop', this.boundDrop);
    fileInput.addEventListener('change', this.boundChange);
  }

  onFile(callback: (file: File) => void): void {
    this.onFileCallback = callback;
  }

  dispose(): void {
    this.dropZone.removeEventListener('dragover', this.boundDragover);
    this.dropZone.removeEventListener('dragleave', this.boundDragleave);
    this.dropZone.removeEventListener('drop', this.boundDrop);
    this.fileInput.removeEventListener('change', this.boundChange);
  }
}
