// A counter that changes when the language does. `t()` reads it, so every text a component shows through `t()`
// is drawn again when the language is switched in place (see language.ts).
export const language = $state({ version: 0 });
