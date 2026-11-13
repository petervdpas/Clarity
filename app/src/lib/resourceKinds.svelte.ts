// The types a resource can have, with their extensions, as the backend
// lists them (clarity_spec::resources::kinds): loaded once, for the project
// tree, the resource view, the New resource dialog and the file pickers.

import { invoke } from "@tauri-apps/api/core";
import type { ResourceKindInfo } from "@clarity/ui/generated/ResourceKindInfo.ts";

export const resourceKinds = $state<{ list: ResourceKindInfo[] }>({ list: [] });

invoke<ResourceKindInfo[] | null>("resource_kinds")
  // A backend that gives no list (an older one, a stand-in in a test) leaves none: every resource is a plain file.
  .then((list) => {
    resourceKinds.list = Array.isArray(list) ? list : [];
  })
  .catch(() => (resourceKinds.list = []));
