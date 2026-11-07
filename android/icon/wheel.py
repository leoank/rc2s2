"""The app icon: a wheel after the Sun Temple's at Konark, as Android vector drawables.

Run from anywhere: `python3 android/icon/wheel.py`. It rewrites
app/src/main/res/drawable/ic_launcher_foreground.xml and ic_notification.xml; edit the
geometry here, not those files.
"""
import math
import pathlib

C = 54.0  # the centre of the 108-unit viewport
R_OUT, R_IN, HUB = 31.0, 27.0, 5.5
EIGHTH = math.pi / 4
UP = -math.pi / 2  # the first broad spoke points straight up
# The launcher wheel at this size leaves room around it in the round mask (radius 36).
LAUNCHER_SCALE = 0.8


def pt(r, a):
    return (C + r * math.cos(a), C + r * math.sin(a))


def f(x):
    return f"{x:.2f}".rstrip("0").rstrip(".")


def dot(x, y, r):
    """A filled circle centred on (x, y), as two arcs."""
    return f"M{f(x - r)},{f(y)} a{f(r)},{f(r)} 0 1,0 {f(2 * r)},0 a{f(r)},{f(r)} 0 1,0 {f(-2 * r)},0"


def ring(r):
    return dot(C, C, r)


def spoke(a, r0, r1, w0, w1):
    """A bar along angle a from radius r0 (half-width w0) to r1 (half-width w1)."""
    nx, ny = -math.sin(a), math.cos(a)
    (x0, y0), (x1, y1) = pt(r0, a), pt(r1, a)
    q = [(x0 + nx * w0, y0 + ny * w0), (x1 + nx * w1, y1 + ny * w1),
         (x1 - nx * w1, y1 - ny * w1), (x0 - nx * w0, y0 - ny * w0)]
    return "M" + " L".join(f"{f(x)},{f(y)}" for x, y in q) + " Z"


# Each part: (fill or stroke, path data, stroke width).
parts = {
    # The rim, a thick ring; inside it a row of sixteen beads, on the sixteenths between spokes.
    "rim": ("stroke", ring((R_OUT + R_IN) / 2), R_OUT - R_IN),
    "beads": ("fill", " ".join(dot(*pt(R_IN - 1.8, i * math.pi / 16), 0.7) for i in range(1, 32, 2)), None),
    # The hub: a boss and a ring around it.
    "hub": ("fill", ring(HUB), None),
    "hubring": ("stroke", ring(8.5), 1.3),
    # Eight broad spokes widening to the rim, each with a medallion halfway; eight thin between.
    "spokes": ("fill", " ".join(spoke(i * EIGHTH + UP, 8.5, R_IN, 1.1, 1.8) for i in range(8)), None),
    "medallions": ("fill", " ".join(dot(*pt(17.5, i * EIGHTH + UP), 2.5) for i in range(8)), None),
    "thin": ("fill", " ".join(spoke(i * EIGHTH + UP + EIGHTH / 2, 8.5, R_IN, 0.4, 0.55) for i in range(8)), None),
}

# Write the launcher foreground and the notification icon.
NAMES = {"rim": "The rim.", "beads": "A row of beads inside it.", "hub": "The hub's boss.",
         "hubring": "The ring around the hub.", "spokes": "Eight broad spokes, widening to the rim.",
         "medallions": "A medallion halfway along each broad spoke.", "thin": "Eight thin spokes between them."}

def body(keys, indent="    "):
    s = ""
    for k in keys:
        kind, d, w = parts[k]
        s += f"{indent}<!-- {NAMES[k]} -->\n"
        if kind == "fill":
            s += f'{indent}<path\n{indent}    android:fillColor="#FFFFFFFF"\n{indent}    android:pathData="{d}" />\n'
        else:
            s += f'{indent}<path\n{indent}    android:strokeColor="#FFFFFFFF"\n{indent}    android:strokeWidth="{w}"\n{indent}    android:pathData="{d}" />\n'
    return s

HEAD = """<?xml version="1.0" encoding="utf-8"?>
<!-- {c} -->
<vector xmlns:android="http://schemas.android.com/apk/res/android"
    android:width="{s}dp"
    android:height="{s}dp"
    android:viewportWidth="108"
    android:viewportHeight="108">
"""
out = pathlib.Path(__file__).resolve().parent.parent / "app/src/main/res/drawable"
(out / "ic_launcher_foreground.xml").write_text(HEAD.format(c="""A wheel after those of the Sun Temple at Konark: a heavy rim with a row of beads, eight
     broad spokes each with a medallion, eight thin spokes between, a hub ringed round.
     White, centred, inside the adaptive icon's 66 dp safe circle. Generated geometry:
     every spoke at an exact eighth or sixteenth of a turn. Scaled down for room around it.""", s=108)
    + f'    <group\n        android:pivotX="54"\n        android:pivotY="54"\n        android:scaleX="{LAUNCHER_SCALE}"\n        android:scaleY="{LAUNCHER_SCALE}">\n'
    + body(["rim", "beads", "thin", "spokes", "medallions", "hubring", "hub"], "        ") + "    </group>\n</vector>\n")
(out / "ic_notification.xml").write_text(HEAD.format(c="The launcher's wheel for the status bar, without the parts too fine to show at 24 dp.", s=24)
    + '    <group\n        android:pivotX="54"\n        android:pivotY="54"\n        android:scaleX="1.65"\n        android:scaleY="1.65">\n'
    + body(["rim", "spokes", "medallions", "hub"], "        ") + "    </group>\n</vector>\n")
