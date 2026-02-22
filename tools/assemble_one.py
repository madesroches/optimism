"""Assemble a single character for quick validation.

Usage:
    blender -b -P tools/assemble_one.py -- soldier
"""

import sys

# Get character name from CLI args after "--"
argv = sys.argv
if "--" in argv:
    char_name = argv[argv.index("--") + 1]
else:
    char_name = "soldier"

# Import and run the full assembly module, but only for one character
import importlib.util
import os

spec = importlib.util.spec_from_file_location(
    "assemble", os.path.join(os.path.dirname(os.path.abspath(__file__)), "assemble_characters.py"))
mod = importlib.util.module_from_spec(spec)
spec.loader.exec_module(mod)

os.makedirs(mod.OUTPUT_DIR, exist_ok=True)

# Find matching character config
for cfg in mod.CHARACTERS + mod.CANDIDE_VARIANTS:
    if cfg["name"] == char_name:
        if char_name.startswith("candide_") and char_name != "candide_base":
            mod.assemble_candide_variant(cfg)
        else:
            mod.assemble_character(cfg)
        break
else:
    print(f"ERROR: Unknown character '{char_name}'")
    print(f"Available: {[c['name'] for c in mod.CHARACTERS + mod.CANDIDE_VARIANTS]}")
    sys.exit(1)
