"""Blender headless script to inspect Quaternius assets.

Run with:
    blender -b -P tools/inspect_assets.py

Inspects the base character model, animation libraries, and outfit pieces.
Reports bone hierarchy, animation actions, and compatibility info.
"""

import bpy
import os
import sys
import json

BASE_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
QUAT_DIR = os.path.join(BASE_DIR, "art", "quaternius")


def clear_scene():
    """Remove all objects from the scene."""
    bpy.ops.object.select_all(action='SELECT')
    bpy.ops.object.delete()
    for collection in bpy.data.collections:
        bpy.data.collections.remove(collection)
    # Clear orphan data
    for block in bpy.data.meshes:
        bpy.data.meshes.remove(block)
    for block in bpy.data.armatures:
        bpy.data.armatures.remove(block)
    for block in bpy.data.actions:
        bpy.data.actions.remove(block)


def import_gltf(filepath):
    """Import a glTF/GLB file."""
    bpy.ops.import_scene.gltf(filepath=filepath)


def import_fbx(filepath):
    """Import an FBX file."""
    bpy.ops.import_scene.fbx(filepath=filepath)


def print_header(title):
    print(f"\n{'='*60}")
    print(f"  {title}")
    print(f"{'='*60}")


def inspect_armature(armature_obj):
    """Print bone hierarchy of an armature."""
    if not armature_obj or armature_obj.type != 'ARMATURE':
        print("  No armature found!")
        return

    armature = armature_obj.data
    print(f"  Armature: {armature_obj.name}")
    print(f"  Total bones: {len(armature.bones)}")

    # Print hierarchy (root bones first, then children indented)
    def print_bone(bone, indent=2):
        prefix = " " * indent
        print(f"{prefix}- {bone.name}")
        for child in bone.children:
            print_bone(child, indent + 2)

    print("  Bone hierarchy:")
    for bone in armature.bones:
        if bone.parent is None:
            print_bone(bone, indent=4)


def inspect_animations():
    """Print all animation actions in the current blend data."""
    actions = bpy.data.actions
    print(f"  Total actions: {len(actions)}")
    for action in sorted(actions, key=lambda a: a.name):
        frame_range = action.frame_range
        num_frames = int(frame_range[1] - frame_range[0]) + 1
        print(f"    - {action.name}: frames {int(frame_range[0])}-{int(frame_range[1])} ({num_frames} frames)")


def inspect_meshes():
    """Print info about mesh objects."""
    meshes = [obj for obj in bpy.data.objects if obj.type == 'MESH']
    print(f"  Mesh objects: {len(meshes)}")
    total_tris = 0
    for mesh_obj in sorted(meshes, key=lambda m: m.name):
        mesh = mesh_obj.data
        # Calculate triangles (each polygon with n verts = n-2 triangles)
        tris = sum(len(p.vertices) - 2 for p in mesh.polygons)
        total_tris += tris
        # Materials
        mat_names = [slot.material.name if slot.material else "None" for slot in mesh_obj.material_slots]
        print(f"    - {mesh_obj.name}: {tris} tris, {len(mesh.vertices)} verts, materials: {mat_names}")

        # Check if parented to armature
        if mesh_obj.parent and mesh_obj.parent.type == 'ARMATURE':
            print(f"      Parented to armature: {mesh_obj.parent.name}")
            # Check for armature modifier
            for mod in mesh_obj.modifiers:
                if mod.type == 'ARMATURE':
                    print(f"      Armature modifier: {mod.object.name if mod.object else 'None'}")
    print(f"  Total triangles: {total_tris}")


def find_file(base_path, filename_pattern):
    """Find a file matching a pattern in a directory tree."""
    import fnmatch
    for root, dirs, files in os.walk(base_path):
        for f in files:
            if fnmatch.fnmatch(f, filename_pattern):
                return os.path.join(root, f)
    return None


# ============================================================
# 1. Inspect Base Character
# ============================================================
print_header("1. BASE CHARACTER - Superhero Male FullBody")
clear_scene()

base_char_path = os.path.join(
    QUAT_DIR, "base-characters",
    "Universal Base Characters[Standard]",
    "Base Characters", "Godot - UE",
    "Superhero_Male_FullBody.gltf"
)

if os.path.exists(base_char_path):
    import_gltf(base_char_path)
    print(f"  Loaded: {base_char_path}")

    # Find armature
    armature = None
    for obj in bpy.data.objects:
        if obj.type == 'ARMATURE':
            armature = obj
            break

    inspect_armature(armature)
    inspect_meshes()
    inspect_animations()

    # Store bone names for comparison
    if armature:
        base_bones = set(b.name for b in armature.data.bones)
        print(f"\n  Key bones for attachment:")
        for name in ['Head', 'Neck', 'Spine', 'Spine1', 'Spine2',
                     'RightHand', 'LeftHand', 'Hips',
                     'RightUpLeg', 'LeftUpLeg']:
            found = [b for b in base_bones if name.lower() in b.lower()]
            print(f"    {name}: {found if found else 'NOT FOUND'}")
else:
    print(f"  FILE NOT FOUND: {base_char_path}")
    base_bones = set()


# ============================================================
# 2. Inspect Animation Library 1
# ============================================================
print_header("2. ANIMATION LIBRARY 1")
clear_scene()

anim1_path = os.path.join(
    QUAT_DIR, "animation-library",
    "Universal Animation Library[Standard]",
    "Unreal-Godot", "UAL1_Standard.glb"
)

if os.path.exists(anim1_path):
    import_gltf(anim1_path)
    print(f"  Loaded: {anim1_path}")

    armature = None
    for obj in bpy.data.objects:
        if obj.type == 'ARMATURE':
            armature = obj
            break

    if armature:
        anim_bones = set(b.name for b in armature.data.bones)
        print(f"  Total bones: {len(anim_bones)}")

        # Check bone compatibility with base character
        if base_bones:
            common = base_bones & anim_bones
            only_base = base_bones - anim_bones
            only_anim = anim_bones - base_bones
            print(f"  Bone compatibility with base character:")
            print(f"    Common bones: {len(common)}")
            print(f"    Only in base: {len(only_base)} - {sorted(only_base)[:10]}")
            print(f"    Only in anim: {len(only_anim)} - {sorted(only_anim)[:10]}")

    inspect_animations()

    # Check for root motion on walk animations
    print("\n  Root motion check (walk animations):")
    for action in bpy.data.actions:
        if 'walk' in action.name.lower() or 'run' in action.name.lower():
            # Check if Hips/Root bone has location keyframes
            has_root_motion = False
            for fcurve in action.fcurves:
                if ('hip' in fcurve.data_path.lower() or 'root' in fcurve.data_path.lower()) \
                   and fcurve.data_path.endswith('location'):
                    # Check if location actually changes
                    values = [kp.co[1] for kp in fcurve.keyframe_points]
                    if len(values) > 1 and (max(values) - min(values)) > 0.01:
                        has_root_motion = True
                        print(f"    {action.name}: ROOT MOTION detected on {fcurve.data_path}[{fcurve.array_index}] "
                              f"(range: {min(values):.3f} to {max(values):.3f})")
            if not has_root_motion:
                print(f"    {action.name}: in-place (no root motion)")
else:
    print(f"  FILE NOT FOUND: {anim1_path}")


# ============================================================
# 3. Inspect Animation Library 2
# ============================================================
print_header("3. ANIMATION LIBRARY 2")
clear_scene()

anim2_path = os.path.join(
    QUAT_DIR, "animation-library-2",
    "Universal Animation Library 2[Standard]",
    "Unreal-Godot", "UAL2_Standard.glb"
)

if os.path.exists(anim2_path):
    import_gltf(anim2_path)
    print(f"  Loaded: {anim2_path}")
    inspect_animations()
else:
    print(f"  FILE NOT FOUND: {anim2_path}")


# ============================================================
# 4. Inspect Outfit Pieces
# ============================================================
print_header("4. MODULAR OUTFIT PIECES")

outfit_base = os.path.join(
    QUAT_DIR, "outfits-fantasy",
    "Modular Character Outfits - Fantasy[Standard]",
    "Exports", "glTF (Godot-Unreal)"
)

# Check full outfits
for outfit_type in ["Male_Peasant", "Male_Ranger"]:
    clear_scene()
    outfit_path = os.path.join(outfit_base, "Outfits", f"{outfit_type}.gltf")
    if os.path.exists(outfit_path):
        import_gltf(outfit_path)
        print(f"\n  Outfit: {outfit_type}")
        inspect_meshes()

        # Check if outfit has its own armature
        outfit_armature = None
        for obj in bpy.data.objects:
            if obj.type == 'ARMATURE':
                outfit_armature = obj
                break

        if outfit_armature:
            outfit_bones = set(b.name for b in outfit_armature.data.bones)
            print(f"  Armature bones: {len(outfit_bones)}")
            if base_bones:
                common = base_bones & outfit_bones
                print(f"  Bone overlap with base: {len(common)}/{len(base_bones)}")
    else:
        print(f"  FILE NOT FOUND: {outfit_path}")

# Check modular parts
print("\n  Available modular parts:")
parts_dir = os.path.join(outfit_base, "Modular Parts")
if os.path.exists(parts_dir):
    parts = sorted([f for f in os.listdir(parts_dir) if f.startswith("Male_") and f.endswith(".gltf")])
    for p in parts:
        print(f"    - {p}")


# ============================================================
# 5. Summary and Recommendations
# ============================================================
print_header("5. SUMMARY")
print("""
  Available for character differentiation:
  - 2 outfit sets: Peasant (plain) and Ranger (hooded, armored)
  - Modular parts: Arms, Body, Feet, Legs, Head_Hood, Acc_Pauldron
  - Material/color swaps on any piece
  - 24 medieval weapons for hand attachment
  - Props: Mug, Chain_Coil, Coin_Pile, etc. for Candide variants

  Character mapping (revised for Standard tier):
  - Candide: Base model, no outfit (or minimal Peasant)
  - Soldier: Ranger outfit + Pauldron + weapon, red materials
  - Inquisitor: Peasant body + Ranger hood (mixed), purple materials
  - Thief: Ranger Hood + light body parts, yellow materials
  - Brute: Full Peasant outfit (bulky), green materials + chain prop
""")
