// Sends one designer edit to the backend. The backend checks the file
// version, applies the edit, runs the checker over the whole project and
// writes the file; any refusal comes back as the error message.

import { invoke } from "@tauri-apps/api/core";
import type { Edit } from "./generated/Edit";
import type { EditApplied } from "./generated/EditApplied";
import type { SpecFile } from "./generated/SpecFile";

export async function applyEdit(file: SpecFile, edit: Edit): Promise<EditApplied> {
  return invoke<EditApplied>("apply_edit", { path: file.path, version: file.version, edit });
}
