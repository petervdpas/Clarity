// Sends one designer edit to the backend. The backend checks the file
// version, applies the edit, runs the checker over the whole project and
// writes the file; any refusal comes back as the error message.

import { invoke } from "@tauri-apps/api/core";
import type { Edit } from "@clarity/ui/generated/Edit.ts";
import type { EditApplied } from "@clarity/ui/generated/EditApplied.ts";
import type { SpecFile } from "@clarity/ui/generated/SpecFile.ts";

export async function applyEdit(file: SpecFile, edit: Edit): Promise<EditApplied> {
  return invoke<EditApplied>("apply_edit", { path: file.path, version: file.version, edit });
}
