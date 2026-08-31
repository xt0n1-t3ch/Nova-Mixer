import type { Action } from "svelte/action";

const FOCUSABLE =
  'a[href], button:not([disabled]), textarea:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])';

/**
 * Keeps Tab inside a modal surface and returns focus to whatever opened it.
 *
 * Without the restore step, dismissing a dialog drops focus to the document
 * body, which strands keyboard and screen-reader users at the top of the page.
 */
export const focusTrap: Action<HTMLElement> = (node) => {
  const opener = document.activeElement as HTMLElement | null;

  function focusable(): HTMLElement[] {
    return Array.from(node.querySelectorAll<HTMLElement>(FOCUSABLE)).filter(
      (element) => element.offsetParent !== null || element === document.activeElement,
    );
  }

  function onKeydown(event: KeyboardEvent): void {
    if (event.key !== "Tab") return;
    const items = focusable();
    if (items.length === 0) {
      event.preventDefault();
      return;
    }
    const first = items[0];
    const last = items[items.length - 1];
    const active = document.activeElement as HTMLElement | null;

    if (event.shiftKey && (active === first || !node.contains(active))) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && active === last) {
      event.preventDefault();
      first.focus();
    }
  }

  node.addEventListener("keydown", onKeydown);
  queueMicrotask(() => {
    const items = focusable();
    (items[0] ?? node).focus();
  });

  return {
    destroy() {
      node.removeEventListener("keydown", onKeydown);
      opener?.focus?.();
    },
  };
};
