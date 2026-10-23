# Icons

The icons a Clarity application can show (`icon "save"` on a button, the
`icon` part), from [Lucide](https://lucide.dev) 1.50.0 under the ISC license
in `LICENSE`. Clarity keeps every icon; an application carries only the ones
it uses.

`lucide.json` maps each icon's name to one SVG path in a 24 by 24 box, drawn
with a 2 pixel round line, in a plain syntax that the web renderer and
Avalonia both read. `convert.py` makes it from the `icon-nodes.json` of the
`lucide-static` package; its first lines say how.
