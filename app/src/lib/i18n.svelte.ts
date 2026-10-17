// The language the renderer shows texts in. Undefined follows the locale of
// the person using the application; the designer can choose a language to
// see the canvas and Preview in it (null: the texts as written).

export const languageChoice = $state<{ language: string | null | undefined }>({ language: undefined });
