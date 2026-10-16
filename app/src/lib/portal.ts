// Moves an element into another part of the page, found by id. Designers use
// it to put their parts and outline into the app's left sidebar.
//
// The moved element must not be a top-level node of its component: Svelte
// removes a component's top-level nodes by walking its siblings, which breaks
// when one of them has moved. Wrap it in an element that stays in place.

export function portal(node: HTMLElement, targetId: string) {
  if (node.parentElement === null) throw new Error("portal needs an element with a parent");
  document.getElementById(targetId)?.appendChild(node);
  return {
    destroy() {
      node.remove();
    },
  };
}
