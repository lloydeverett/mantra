// Re-trigger a CSS animation even if the class is already present.
export function replay(el, cls) {
  el.classList.remove(cls);
  void el.offsetWidth;
  el.classList.add(cls);
}

export const emit = (el, type, detail) => el.dispatchEvent(new CustomEvent(type, { detail }));

// A Range over characters [start, end) of root's text, which may span several text nodes.
export function textRange(root, start, end) {
  const range = document.createRange();
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
  for (let node, pos = 0; (node = walker.nextNode()); pos += node.length) {
    if (start >= pos && start <= pos + node.length) range.setStart(node, start - pos);
    if (end >= pos && end <= pos + node.length) {
      range.setEnd(node, end - pos);
      return range;
    }
  }
  return null;
}
