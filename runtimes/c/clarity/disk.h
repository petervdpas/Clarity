// Files and folders on the disk, for databases and file stores: the few
// calls the runtime needs, with the runtime's errors.

#pragma once

#include "clarity/core.h"

/// Makes `folder` and the folders above it that are not there yet.
bool clarity_make_folders(const char* folder);
/// The folder `path` is in; "" for a bare name.
const char* clarity_parent(const char* path);
bool clarity_exists(const char* path);
/// The bytes of the file `path`, in the scratch arena.
bool clarity_read_file(const char* path, ClarityBytes* out);
/// Writes the file `path`, replacing what it held.
bool clarity_write_file(const char* path, ClarityBytes bytes);
