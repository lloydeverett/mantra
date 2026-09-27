// Re-trigger a CSS animation even if the class is already present.
export function replay(el, cls) {
  el.classList.remove(cls);
  void el.offsetWidth;
  el.classList.add(cls);
}

export const emit = (el, type, detail) => el.dispatchEvent(new CustomEvent(type, { detail }));
