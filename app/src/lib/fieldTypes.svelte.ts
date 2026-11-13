// The field types as the backend lists them (clarity_spec::dictionary,
// through the catalog): the same list the tools offer agents, loaded once
// for the entity editor and the app's settings.

import { invoke } from "@tauri-apps/api/core";
import type { Catalog } from "@clarity/ui/generated/Catalog.ts";

export const fieldTypes = $state<{ simple: string[]; named: string[]; settings: string[] }>({ simple: [], named: [], settings: [] });

invoke<Catalog>("stack_catalog")
  .then((catalog) => {
    fieldTypes.simple = catalog.field_types;
    fieldTypes.named = catalog.named_types;
    fieldTypes.settings = catalog.setting_types;
  })
  .catch(() => undefined);
