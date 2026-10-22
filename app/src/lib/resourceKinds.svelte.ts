// The types a resource can have, with their extensions, as the backend
// lists them (clarity_spec::resources::kinds): loaded once, for the project
// tree, the resource view, the New resource dialog and the file pickers.

import { invoke } from "@tauri-apps/api/core";
import type { ResourceKindInfo } from "./generated/ResourceKindInfo";

export const resourceKinds = $state<{ list: ResourceKindInfo[] }>({ list: [] });

invoke<ResourceKindInfo[]>("resource_kinds")
  .then((list) => (resourceKinds.list = list))
  .catch(() => (resourceKinds.list = []));
