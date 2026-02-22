"""Blender headless script: assemble all game characters from Quaternius assets.

Run with:
    blender -b -P tools/assemble_characters.py

For each character, this script:
1. Imports the base model or a complete outfit file
2. Imports animations from the animation library
3. Optionally imports extra modular outfit pieces
4. Applies character-specific material colors
5. Saves as art/characters/{name}.blend

All outfit files and base character share the same 65-bone armature,
so animations are fully compatible across all models.
"""

import bpy
import math
import os
import sys

BASE_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
QUAT_DIR = os.path.join(BASE_DIR, "art", "quaternius")
OUTPUT_DIR = os.path.join(BASE_DIR, "art", "characters")

# ---------------------------------------------------------------------------
# Asset paths
# ---------------------------------------------------------------------------

PATHS = {
    "base_character": os.path.join(
        QUAT_DIR, "base-characters",
        "Universal Base Characters[Standard]",
        "Base Characters", "Godot - UE",
        "Superhero_Male_FullBody.gltf",
    ),
    "anim_library_1": os.path.join(
        QUAT_DIR, "animation-library",
        "Universal Animation Library[Standard]",
        "Unreal-Godot", "UAL1_Standard.glb",
    ),
    "outfit_peasant": os.path.join(
        QUAT_DIR, "outfits-fantasy",
        "Modular Character Outfits - Fantasy[Standard]",
        "Exports", "glTF (Godot-Unreal)",
        "Outfits", "Male_Peasant.gltf",
    ),
    "outfit_ranger": os.path.join(
        QUAT_DIR, "outfits-fantasy",
        "Modular Character Outfits - Fantasy[Standard]",
        "Exports", "glTF (Godot-Unreal)",
        "Outfits", "Male_Ranger.gltf",
    ),
}

# Modular parts directory for mix-and-match
PARTS_DIR = os.path.join(
    QUAT_DIR, "outfits-fantasy",
    "Modular Character Outfits - Fantasy[Standard]",
    "Exports", "glTF (Godot-Unreal)",
    "Modular Parts",
)

WEAPONS_DIR = os.path.join(QUAT_DIR, "medieval-weapons", "FBX")
PROPS_DIR = os.path.join(QUAT_DIR, "fantasy-props", "Exports", "glTF")

# ---------------------------------------------------------------------------
# Character definitions
# ---------------------------------------------------------------------------

CHARACTERS = [
    {
        "name": "candide_base",
        "model": "base_character",
        "extra_parts": [],
        "color": (0.9, 0.88, 0.82),  # Warm white / cream
        "description": "Candide — bare base model, unadorned",
    },
    {
        "name": "soldier",
        "model": "outfit_ranger",
        "extra_parts": [],
        "color": (0.75, 0.12, 0.1),  # Deep red
        "weapon": "Sword.fbx",
        "description": "Soldier — full ranger armor, red, with sword",
    },
    {
        "name": "inquisitor",
        "model": "outfit_peasant",
        "extra_parts": ["Male_Ranger_Head_Hood.gltf"],
        "color": (0.45, 0.12, 0.65),  # Purple
        "description": "Inquisitor — peasant robes + ranger hood, purple",
    },
    {
        "name": "thief",
        "model": "outfit_ranger",
        "extra_parts": [],
        "color": (0.85, 0.72, 0.08),  # Gold/yellow
        "remove_parts": ["Male_Ranger_Acc_Pauldron"],  # lighter silhouette
        "description": "Thief — ranger outfit without pauldron, gold",
    },
    {
        "name": "brute",
        "model": "outfit_peasant",
        "extra_parts": [],
        "color": (0.18, 0.55, 0.12),  # Dark green
        "description": "Brute — full peasant outfit, green",
    },
]

# Candide luxury variants — disabled for now, props too small at 64x64
# TODO: revisit when we find a better approach (glow effects, larger props, higher res)
CANDIDE_VARIANTS = []


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def clear_scene():
    """Completely clear the scene and all data blocks."""
    # Delete all objects
    bpy.ops.object.select_all(action='SELECT')
    bpy.ops.object.delete()

    # Remove orphan data
    for block_type in (bpy.data.meshes, bpy.data.armatures, bpy.data.actions,
                       bpy.data.materials, bpy.data.images, bpy.data.cameras,
                       bpy.data.lights, bpy.data.collections):
        for block in list(block_type):
            block_type.remove(block)


def get_object_names():
    """Return a set of all current object names."""
    return set(obj.name for obj in bpy.data.objects)


def import_gltf(filepath):
    """Import a glTF/GLB file. Returns set of newly created object names."""
    before = get_object_names()
    bpy.ops.import_scene.gltf(filepath=filepath)
    after = get_object_names()
    return after - before


def import_fbx(filepath):
    """Import an FBX file. Returns set of newly created object names."""
    before = get_object_names()
    bpy.ops.import_scene.fbx(filepath=filepath)
    after = get_object_names()
    return after - before


def find_armature(names=None):
    """Find the first armature in the scene, optionally filtering by name set."""
    for obj in bpy.data.objects:
        if obj.type == 'ARMATURE':
            if names is None or obj.name in names:
                return obj
    return None


def find_armatures(names):
    """Find all armatures whose names are in the given set."""
    return [obj for obj in bpy.data.objects if obj.type == 'ARMATURE' and obj.name in names]


def find_meshes(names):
    """Find all mesh objects whose names are in the given set."""
    return [obj for obj in bpy.data.objects if obj.type == 'MESH' and obj.name in names]


def reparent_meshes_to_armature(mesh_names, target_armature):
    """Re-parent mesh objects to a different armature."""
    for name in mesh_names:
        obj = bpy.data.objects.get(name)
        if obj is None or obj.type != 'MESH':
            continue

        # Update parent
        obj.parent = target_armature

        # Update armature modifier
        for mod in obj.modifiers:
            if mod.type == 'ARMATURE':
                mod.object = target_armature


def delete_objects(names):
    """Delete objects by name."""
    for name in list(names):
        obj = bpy.data.objects.get(name)
        if obj:
            bpy.data.objects.remove(obj, do_unlink=True)


def remove_icospheres():
    """Remove any Icosphere objects (debug objects from Quaternius exports)."""
    for obj in list(bpy.data.objects):
        if 'icosphere' in obj.name.lower():
            bpy.data.objects.remove(obj, do_unlink=True)


def apply_color_to_materials(color_rgb, exclude_materials=None):
    """Set the base color of all materials to the given RGB color.

    Disconnects any texture nodes feeding into Base Color so the flat
    color actually takes effect. Preserves skin/eye materials if specified.
    """
    exclude = set(exclude_materials or [])
    for mat in bpy.data.materials:
        if mat.name in exclude:
            continue
        if not mat.use_nodes:
            continue
        tree = mat.node_tree
        for node in tree.nodes:
            if node.type == 'BSDF_PRINCIPLED':
                bc_input = node.inputs['Base Color']
                # Remove any links into Base Color (textures, MIX nodes, etc.)
                for link in list(tree.links):
                    if link.to_socket == bc_input:
                        tree.links.remove(link)
                bc_input.default_value = (*color_rgb, 1.0)


def mark_actions_persistent():
    """Set fake_user on all actions so they persist in the .blend file."""
    for action in bpy.data.actions:
        action.use_fake_user = True


def create_simple_prop(name, shape="cube", size=0.05, color=(0.85, 0.7, 0.1)):
    """Create a simple geometric prop (for luxury items we can't source)."""
    if shape == "cube":
        bpy.ops.mesh.primitive_cube_add(size=size)
    elif shape == "cylinder":
        bpy.ops.mesh.primitive_cylinder_add(radius=size / 2, depth=size)
    elif shape == "torus":
        bpy.ops.mesh.primitive_torus_add(major_radius=size, minor_radius=size * 0.3)

    obj = bpy.context.active_object
    obj.name = name

    # Gold material
    mat = bpy.data.materials.new(f"MI_{name}")
    mat.use_nodes = True
    for node in mat.node_tree.nodes:
        if node.type == 'BSDF_PRINCIPLED':
            node.inputs['Base Color'].default_value = (*color, 1.0)
            node.inputs['Metallic'].default_value = 0.9
            node.inputs['Roughness'].default_value = 0.3
    obj.data.materials.append(mat)

    return obj


def parent_to_bone(obj, armature, bone_name, offset=(0, 0, 0)):
    """Parent an object to a specific bone on an armature."""
    obj.parent = armature
    obj.parent_type = 'BONE'
    obj.parent_bone = bone_name
    obj.location = offset


# ---------------------------------------------------------------------------
# Assembly pipeline
# ---------------------------------------------------------------------------

def assemble_character(config):
    """Build a single character .blend file."""
    name = config["name"]
    print(f"\n{'='*60}")
    print(f"  Assembling: {name}")
    print(f"  {config.get('description', '')}")
    print(f"{'='*60}")

    clear_scene()

    # Step 1: Import main model
    model_path = PATHS[config["model"]]
    print(f"  Importing model: {os.path.basename(model_path)}")
    new_objs = import_gltf(model_path)
    main_armature = find_armature()
    if main_armature is None:
        print(f"  ERROR: No armature found after importing {model_path}")
        return False
    main_armature_name = main_armature.name
    print(f"  Main armature: {main_armature_name}")

    # Step 2: Import animation library
    print(f"  Importing animations...")
    anim_objs = import_gltf(PATHS["anim_library_1"])
    # Delete animation library's objects (armature + meshes), keep actions
    delete_objects(anim_objs)
    mark_actions_persistent()
    print(f"  Loaded {len(bpy.data.actions)} animation actions")

    # Step 3: Import extra outfit parts
    for part_name in config.get("extra_parts", []):
        part_path = os.path.join(PARTS_DIR, part_name)
        if not os.path.exists(part_path):
            print(f"  WARNING: Part not found: {part_path}")
            continue

        print(f"  Importing part: {part_name}")
        part_objs = import_gltf(part_path)

        # Find the part's armature and meshes
        part_armatures = find_armatures(part_objs)
        part_meshes = [n for n in part_objs
                       if bpy.data.objects.get(n) and bpy.data.objects[n].type == 'MESH']

        # Re-parent meshes to main armature
        reparent_meshes_to_armature(part_meshes, main_armature)

        # Delete part's armature(s)
        for arm in part_armatures:
            bpy.data.objects.remove(arm, do_unlink=True)

    # Step 4: Remove unwanted parts
    for part_pattern in config.get("remove_parts", []):
        for obj in list(bpy.data.objects):
            if part_pattern.lower() in obj.name.lower() and obj.type == 'MESH':
                print(f"  Removing: {obj.name}")
                bpy.data.objects.remove(obj, do_unlink=True)

    # Step 5: Import weapon (if specified)
    if "weapon" in config:
        weapon_path = os.path.join(WEAPONS_DIR, config["weapon"])
        if os.path.exists(weapon_path):
            print(f"  Importing weapon: {config['weapon']}")
            weapon_objs = import_fbx(weapon_path)
            # Find the weapon mesh and parent it to right hand bone
            for wname in weapon_objs:
                obj = bpy.data.objects.get(wname)
                if obj and obj.type == 'MESH':
                    parent_to_bone(obj, main_armature, "hand_r", offset=(0, 0, 0.1))
                elif obj and obj.type == 'ARMATURE':
                    bpy.data.objects.remove(obj, do_unlink=True)

    # Step 6: Clean up debug objects
    remove_icospheres()

    # Step 7: Apply character color
    # Keep skin/eye materials as-is, recolor outfit materials
    skin_materials = {"MI_Eyes", "MI_Hair_1", "MI_Regular_Male", "MI_Superhero_Male"}
    apply_color_to_materials(config["color"], exclude_materials=skin_materials)

    # Step 8: Save
    output_path = os.path.join(OUTPUT_DIR, f"{name}.blend")
    bpy.ops.wm.save_as_mainfile(filepath=output_path)
    print(f"  Saved: {output_path}")
    return True


def assemble_candide_variant(config):
    """Build a Candide luxury variant .blend file."""
    name = config["name"]
    print(f"\n{'='*60}")
    print(f"  Assembling variant: {name}")
    print(f"  {config.get('description', '')}")
    print(f"{'='*60}")

    clear_scene()

    # Import base character
    print(f"  Importing base character...")
    import_gltf(PATHS["base_character"])
    main_armature = find_armature()

    # Import animations
    print(f"  Importing animations...")
    anim_objs = import_gltf(PATHS["anim_library_1"])
    delete_objects(anim_objs)
    mark_actions_persistent()

    remove_icospheres()

    # Create or import prop
    prop_source = config.get("prop_source", "")
    prop_bone = config["prop_bone"]

    if prop_source == "props":
        # Create a simple geometric prop
        if "grill" in name:
            prop = create_simple_prop("Grill", shape="cube", size=0.08,
                                      color=(0.85, 0.7, 0.1))
        elif "chain" in name:
            prop = create_simple_prop("Chain", shape="torus", size=0.15,
                                      color=(0.85, 0.7, 0.1))
        else:
            prop = create_simple_prop("Prop", shape="cube", size=0.1,
                                      color=(0.85, 0.7, 0.1))
        parent_to_bone(prop, main_armature, prop_bone)
    else:
        # Try to import from fantasy props
        prop_path = os.path.join(PROPS_DIR, f"{prop_source}.gltf")
        if os.path.exists(prop_path):
            print(f"  Importing prop: {prop_source}")
            prop_objs = import_gltf(prop_path)
            for pname in prop_objs:
                obj = bpy.data.objects.get(pname)
                if obj and obj.type == 'MESH':
                    parent_to_bone(obj, main_armature, prop_bone)
                    # Make it gold
                    gold_mat = bpy.data.materials.new(f"MI_Gold_{name}")
                    gold_mat.use_nodes = True
                    for node in gold_mat.node_tree.nodes:
                        if node.type == 'BSDF_PRINCIPLED':
                            node.inputs['Base Color'].default_value = (0.85, 0.7, 0.1, 1.0)
                            node.inputs['Metallic'].default_value = 0.9
                    obj.data.materials.clear()
                    obj.data.materials.append(gold_mat)
        else:
            print(f"  WARNING: Prop not found, using placeholder: {prop_path}")
            prop = create_simple_prop(prop_source, shape="cube", size=0.1,
                                      color=(0.85, 0.7, 0.1))
            parent_to_bone(prop, main_armature, prop_bone)

    # Apply base color (cream/white for Candide)
    skin_materials = {"MI_Eyes", "MI_Hair_1"}
    apply_color_to_materials(config["color"], exclude_materials=skin_materials)

    output_path = os.path.join(OUTPUT_DIR, f"{name}.blend")
    bpy.ops.wm.save_as_mainfile(filepath=output_path)
    print(f"  Saved: {output_path}")
    return True


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    os.makedirs(OUTPUT_DIR, exist_ok=True)

    # Verify key asset paths exist
    missing = []
    for key, path in PATHS.items():
        if not os.path.exists(path):
            missing.append(f"  {key}: {path}")
    if missing:
        print("ERROR: Missing asset files:")
        for m in missing:
            print(m)
        sys.exit(1)

    success = 0
    fail = 0

    # Assemble main characters
    for config in CHARACTERS:
        if assemble_character(config):
            success += 1
        else:
            fail += 1

    # Assemble Candide variants
    for config in CANDIDE_VARIANTS:
        if assemble_candide_variant(config):
            success += 1
        else:
            fail += 1

    print(f"\n{'='*60}")
    print(f"  DONE: {success} characters assembled, {fail} failures")
    print(f"{'='*60}")

    if fail > 0:
        sys.exit(1)


if __name__ == "__main__":
    main()
