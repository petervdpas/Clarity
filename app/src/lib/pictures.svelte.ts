// The pictures the renderer shows, reactive: a part showing one draws again
// when it has loaded. See pictures.ts.

import { Pictures, type PictureLoader } from "./pictures";

let version = $state(0);
let pictures = new Pictures(null, () => version++);

/** The URL of a picture of the resources folder, as pictures.ts gives it. */
export function pictureUrl(name: string): string | null | undefined {
  void version;
  return pictures.url(name);
}

/** Loads pictures with `load` from now on, forgetting the ones loaded. */
export function loadPicturesWith(load: PictureLoader | null) {
  pictures = new Pictures(load, () => version++);
  version++;
}
