// The popovers opened in a preview, where no application runs, by name; a
// running application keeps them in its parts' state instead.
export const previewPopovers = $state<Record<string, boolean>>({});
