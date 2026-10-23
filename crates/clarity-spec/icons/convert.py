"""Makes lucide.json from Lucide's icon-nodes.json (the lucide-static package).

Every icon becomes one SVG path in plain syntax: each command letter and
number separated by a space, arcs with their flags written out, and circles,
rectangles, lines and polygons drawn as paths. The web renderer draws it as
an SVG path and Avalonia as a Path geometry, so both read the same text.

    npm pack lucide-static@<version> && tar xzf lucide-static-<version>.tgz
    python3 convert.py package/icon-nodes.json > lucide.json
"""

import json
import re
import sys

NUMBER = re.compile(r"[+-]?(?:\d+\.?\d*|\.\d+)(?:[eE][+-]?\d+)?")
ARGS = {"M": 2, "L": 2, "H": 1, "V": 1, "C": 6, "S": 4, "Q": 4, "T": 2, "A": 7, "Z": 0}


def fmt(n):
    text = f"{n:.3f}".rstrip("0").rstrip(".")
    return "0" if text in ("-0", "") else text


def plain(d):
    """A path's data with every number and flag on its own, and the command
    letter written before each group of its numbers (a repeated move is a line)."""
    out = []
    i = 0
    command = None
    count = 0
    previous = ""
    while i < len(d):
        c = d[i]
        if c.isalpha():
            command = c
            count = 0
            if ARGS[c.upper()] == 0:
                out.append(c)
                previous = c
            i += 1
            continue
        if c in " ,\t\n":
            i += 1
            continue
        if command is None:
            raise ValueError(f"number before a command in {d!r}")
        size = ARGS[command.upper()]
        if count % size == 0:
            letter = command
            if count > 0 and command in "Mm":
                letter = "L" if command == "M" else "l"
            # A smooth curve after anything but a curve starts at the current
            # point; Avalonia reflects an older control point instead, so it
            # is written as the curve it is.
            if letter in "sStT" and previous.upper() not in ("CS" if letter in "sS" else "QT"):
                if letter.isupper():
                    raise ValueError(f"an absolute smooth curve after a move in {d!r}")
                letter = "c" if letter == "s" else "q"
                out.append(letter)
                out.append("0")
                out.append("0")
                previous = letter
            else:
                out.append(letter)
                previous = letter
        # An arc's flags are single digits that may touch what follows.
        if command.upper() == "A" and count % 7 in (3, 4):
            out.append(c)
            count += 1
            i += 1
            continue
        m = NUMBER.match(d, i)
        if not m:
            raise ValueError(f"unreadable {d[i:]!r}")
        out.append(fmt(float(m.group())))
        count += 1
        i = m.end()
    return " ".join(out)


def num(attrs, key, default=0.0):
    return float(attrs.get(key, default))


def node_path(tag, a):
    if tag == "path":
        # A path's first move is absolute even when written `m`; joined after
        # other shapes it would not be.
        d = plain(a["d"])
        return "M" + d[1:] if d.startswith("m") else d
    if tag in ("circle", "ellipse"):
        cx, cy = num(a, "cx"), num(a, "cy")
        rx = num(a, "r") if tag == "circle" else num(a, "rx")
        ry = num(a, "r") if tag == "circle" else num(a, "ry")
        return f"M {fmt(cx - rx)} {fmt(cy)} A {fmt(rx)} {fmt(ry)} 0 1 0 {fmt(cx + rx)} {fmt(cy)} A {fmt(rx)} {fmt(ry)} 0 1 0 {fmt(cx - rx)} {fmt(cy)} Z"
    if tag == "rect":
        x, y, w, h = num(a, "x"), num(a, "y"), num(a, "width"), num(a, "height")
        rx = num(a, "rx", a.get("ry", 0))
        ry = num(a, "ry", a.get("rx", 0))
        if rx == 0 and ry == 0:
            return f"M {fmt(x)} {fmt(y)} H {fmt(x + w)} V {fmt(y + h)} H {fmt(x)} Z"
        return (
            f"M {fmt(x + rx)} {fmt(y)} H {fmt(x + w - rx)} A {fmt(rx)} {fmt(ry)} 0 0 1 {fmt(x + w)} {fmt(y + ry)} "
            f"V {fmt(y + h - ry)} A {fmt(rx)} {fmt(ry)} 0 0 1 {fmt(x + w - rx)} {fmt(y + h)} "
            f"H {fmt(x + rx)} A {fmt(rx)} {fmt(ry)} 0 0 1 {fmt(x)} {fmt(y + h - ry)} "
            f"V {fmt(y + ry)} A {fmt(rx)} {fmt(ry)} 0 0 1 {fmt(x + rx)} {fmt(y)} Z"
        )
    if tag == "line":
        return f"M {fmt(num(a, 'x1'))} {fmt(num(a, 'y1'))} L {fmt(num(a, 'x2'))} {fmt(num(a, 'y2'))}"
    if tag in ("polyline", "polygon"):
        points = [fmt(float(p)) for p in NUMBER.findall(a["points"])]
        pairs = [f"{points[i]} {points[i + 1]}" for i in range(0, len(points), 2)]
        return "M " + " L ".join(pairs) + (" Z" if tag == "polygon" else "")
    raise ValueError(f"unknown element {tag}")


def main():
    nodes = json.load(open(sys.argv[1]))
    icons = {name: " ".join(node_path(tag, attrs) for tag, attrs in parts) for name, parts in sorted(nodes.items())}
    json.dump(icons, sys.stdout, indent=0, separators=(",", ":"))
    sys.stdout.write("\n")


main()
