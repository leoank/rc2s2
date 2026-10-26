"""Assemble and build the rc2s2 book (the GitHub Pages site).

    python3 docs/guide/build.py <repo root> <out dir>

Copies docs/guide/ to a temporary directory, adds what is generated so it cannot drift from
the code (the command-line reference from `rc2s2 --help` and `rc2s2-hub --help`, the design
documents from docs/ with one sidebar entry per ADR), turns the notebook's `[[ID]]` links
into links to the page on GitHub, checks every relative link, and runs `mdbook build` into
<out dir>. Needs `rc2s2`, `rc2s2-hub` and `mdbook` on PATH; `nix build .#docs` provides
them.
"""

import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

root, out = Path(sys.argv[1]).resolve(), Path(sys.argv[2]).resolve()
work = Path(tempfile.mkdtemp(prefix="rc2s2-book-"))
shutil.copytree(root / "docs/guide", work, dirs_exist_ok=True, ignore=shutil.ignore_patterns("build.py"))
for p in [work, *work.rglob("*")]:  # sources from the Nix store are read-only
    p.chmod(p.stat().st_mode | 0o200)
src = work / "src"
REPO = "https://github.com/leoank/rc2s2/blob/main"


def help_text(*args):
    return subprocess.run([*args, "--help"], check=True, capture_output=True, text=True).stdout


def subcommands(text):
    """Names under `Commands:` in clap's help, without `help`."""
    names, inside = [], False
    for line in text.splitlines():
        if line.startswith("Commands:"):
            inside = True
        elif inside and not line.strip():
            break
        elif inside:
            name = line.split()[0]
            if name != "help":
                names.append(name)
    return names


# --- command-line reference ------------------------------------------------------------
cli = [
    "# Command line\n",
    "Generated from `--help` at build time, so it always matches the binaries the site was",
    "built with. `rc2s2` is the node daemon, the terminal wrapper and a command-line client;",
    "`rc2s2-hub` is the hub.\n",
]


def walk(cmd, level):
    text = help_text(*cmd)
    cli.extend([f"{'#' * level} {' '.join(cmd)}\n", "```text", text.rstrip(), "```\n"])
    for sub in subcommands(text):
        walk([*cmd, sub], min(level + 1, 4))


walk(["rc2s2"], 2)
walk(["rc2s2-hub"], 2)
(src / "reference/cli.md").write_text("\n".join(cli))

# --- design documents ------------------------------------------------------------------
design = src / "design"
(design / "decisions").mkdir(parents=True, exist_ok=True)
shutil.copyfile(root / "docs/architecture.md", design / "architecture.md")
adrs = []
for path in sorted((root / "docs/decisions").glob("*.md")):
    shutil.copyfile(path, design / "decisions" / path.name)
    if path.name.startswith("ADR-"):
        title = path.read_text().splitlines()[0].lstrip("# ").strip()
        adrs.append(f"    - [{title}](design/decisions/{path.name})")
summary = src / "SUMMARY.md"
summary.write_text(summary.read_text().replace("<!-- ADRS -->", "\n".join(adrs)))

# --- notebook links --------------------------------------------------------------------
# The design documents cite the project's notebook as `[[ID]]`; on the site each becomes a
# link to that page on GitHub, or plain code when the page lives in another notebook.
pages = {}
for path in (root / "notes").rglob("*.md"):
    m = re.match(r"([A-Z]{2,4}-[0-9A-Za-z]+(?:-[0-9a-z]{6})?)", path.name)
    if m:
        pages.setdefault(m.group(1), path)
pages["PRJ-0001"] = root / "notes/README.md"
WIKI = re.compile(r"\[\[([^\]|]+)(?:\|([^\]]+))?\]\]")


def wiki(m):
    target, label = m.group(1), m.group(2) or m.group(1)
    page = pages.get(target)
    if page is None:
        return f"`{label}`"
    return f"[{label}]({REPO}/{page.relative_to(root)})"


for page in design.rglob("*.md"):
    page.write_text(WIKI.sub(wiki, page.read_text()))

# --- every relative link resolves ------------------------------------------------------
LINK = re.compile(r"\]\(([^)\s]+)\)")
broken = []
for page in src.rglob("*.md"):
    body = re.sub(r"```.*?```", "", page.read_text(), flags=re.S)
    for target in LINK.findall(body):
        if re.match(r"[a-z]+:", target) or target.startswith("#"):
            continue
        if not (page.parent / target.split("#")[0]).exists():
            broken.append(f"{page.relative_to(src)}: {target}")
if broken:
    sys.exit("broken links:\n  " + "\n  ".join(broken))

subprocess.run(["mdbook", "build", str(work), "--dest-dir", str(out)], check=True)
shutil.rmtree(work)
