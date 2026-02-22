#!/usr/bin/env python3
"""Batch-render all character .blend files into sprite sheets.

Usage:
    python3 tools/render_all.py

Prerequisites:
    - Blender 4.x on PATH
    - Character .blend files in art/characters/
"""

import os
import shutil
import subprocess
import sys
from pathlib import Path


def main():
    project_dir = Path(__file__).resolve().parent.parent
    render_script = project_dir / "tools" / "render_sprites.py"
    characters_dir = project_dir / "art" / "characters"
    output_dir = project_dir / "assets" / "sprites"

    output_dir.mkdir(parents=True, exist_ok=True)

    blender = shutil.which("blender")
    if blender is None:
        print("ERROR: blender not found on PATH", file=sys.stderr)
        sys.exit(1)

    if not characters_dir.is_dir():
        print(f"ERROR: {characters_dir} does not exist", file=sys.stderr)
        sys.exit(1)

    blend_files = sorted(characters_dir.glob("*.blend"))
    if not blend_files:
        print(f"WARNING: No .blend files found in {characters_dir}")
        sys.exit(0)

    fail_count = 0

    for blend in blend_files:
        name = blend.stem
        output = output_dir / f"{name}.png"

        print(f"=== Rendering {name} ===")
        result = subprocess.run(
            [
                blender, "-b", str(blend),
                "-P", str(render_script),
                "--", "--output", str(output),
            ],
            check=False,
        )

        if result.returncode == 0:
            print(f"OK: {output}")
        else:
            print(f"FAILED: {name}", file=sys.stderr)
            fail_count += 1
        print()

    print(f"=== Done: {len(blend_files)} characters, {fail_count} failures ===")
    if fail_count > 0:
        sys.exit(1)


if __name__ == "__main__":
    main()
