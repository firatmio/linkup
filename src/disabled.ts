export function disableWebViewInteractions(): void {
  document.addEventListener(
    'contextmenu',
    (event: MouseEvent): void => {
      event.preventDefault();
    },
    { capture: true }
  );

  window.addEventListener(
    'keydown',
    (event: KeyboardEvent): void => {
      const isMac = navigator.platform.toUpperCase().includes('MAC');
      const modifier = isMac ? event.metaKey : event.ctrlKey;

      const blockedFnKeys = [
        'F3',
        'F5',
        'F7',
        'F11',
        'F12',
      ];

      if (blockedFnKeys.includes(event.key)) {
        event.preventDefault();
        event.stopPropagation();
        return;
      }

      if (modifier) {
        const key = event.key.toLowerCase();

        switch (key) {
          case 'r':
          case 'f5':
          case 'u':
          case 'p':
          case 's':
          case 'g':
          case 'f':
            event.preventDefault();
            event.stopPropagation();
            break;

          case 'i':
          case 'j':
            if (event.shiftKey || event.altKey) {
              event.preventDefault();
              event.stopPropagation();
            }
            break;

          default:
            break;
        }
      }
    },
    { capture: true }
  );

  window.addEventListener(
    'dragover',
    (event: DragEvent): void => {
      event.preventDefault();
    },
    { capture: true }
  );

  window.addEventListener(
    'drop',
    (event: DragEvent): void => {
      event.preventDefault();
    },
    { capture: true }
  );
}