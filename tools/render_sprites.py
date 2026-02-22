"""Blender headless sprite sheet renderer for Optimism.

Usage:
    blender -b art/characters/soldier.blend -P tools/render_sprites.py -- \
        --output assets/sprites/soldier.png

Renders all animation actions on the active armature into a single sprite sheet
with 4-directional variants for movement/combat animations.

Output: PNG sprite sheet + JSON metadata sidecar file.
"""

import bpy
import json
import math
import os
import sys
from mathutils import Euler

# ---------------------------------------------------------------------------
# CLI argument parsing (after Blender's "--" separator)
# ---------------------------------------------------------------------------

def parse_args():
    argv = sys.argv
    if "--" in argv:
        argv = argv[argv.index("--") + 1:]
    else:
        argv = []

    import argparse
    parser = argparse.ArgumentParser(description="Render sprite sheet from Blender file")
    parser.add_argument("--output", required=True, help="Output PNG path (JSON written alongside)")
    parser.add_argument("--size", type=int, default=64, help="Frame size in pixels (default: 64)")
    parser.add_argument("--columns", type=int, default=8, help="Columns in sprite sheet (default: 8)")
    parser.add_argument("--camera-angle", type=float, default=35.0,
                        help="Camera tilt from vertical in degrees (default: 35)")
    parser.add_argument("--camera-distance", type=float, default=4.0,
                        help="Camera distance from origin (default: 4.0)")
    parser.add_argument("--outline", action="store_true",
                        help="Add 1px dark outline to sprites (Freestyle)")
    return parser.parse_args(argv)


# ---------------------------------------------------------------------------
# Animation configuration
# ---------------------------------------------------------------------------

# Map animation action name patterns to rendering config.
# "directional" animations get rendered at 4 rotations (down/left/up/right).
# "static" animations render at default rotation only.
#
# The order here defines the order in the sprite sheet.
# We search for action names containing these substrings (case-insensitive).
ANIMATION_CONFIG = [
    {"name": "walk",   "search": ["walk"],            "directional": True,  "frames": 6},
    {"name": "idle",   "search": ["idle"],            "directional": False, "frames": 2},
    {"name": "attack", "search": ["attack", "melee", "slash"], "directional": True,  "frames": 4},
    {"name": "death",  "search": ["death", "die"],    "directional": False, "frames": 4},
]

# Rotation angles for directional animations (Blender Z-axis rotation in radians).
# Order: down (facing camera), left, up (away from camera), right.
DIRECTION_ROTATIONS = {
    "down":  0.0,
    "left":  math.radians(90),
    "up":    math.radians(180),
    "right": math.radians(270),
}


# ---------------------------------------------------------------------------
# Scene setup
# ---------------------------------------------------------------------------

def setup_scene(args):
    """Configure render settings for sprite output."""
    scene = bpy.context.scene

    # Render settings
    scene.render.engine = "BLENDER_EEVEE_NEXT" if bpy.app.version >= (4, 0, 0) else "BLENDER_EEVEE"
    scene.render.resolution_x = args.size
    scene.render.resolution_y = args.size
    scene.render.resolution_percentage = 100
    scene.render.film_transparent = True
    scene.render.image_settings.file_format = "PNG"
    scene.render.image_settings.color_mode = "RGBA"

    # Samples (low for speed, this is pixel art output)
    if hasattr(scene, "eevee"):
        scene.eevee.taa_render_samples = 16

    # Freestyle outline (optional)
    if args.outline:
        scene.render.use_freestyle = True
        scene.render.line_thickness = 1.0
    else:
        scene.render.use_freestyle = False

    # Remove default objects
    for obj in list(bpy.data.objects):
        if obj.type in ("LIGHT", "CAMERA") and obj.name.startswith(("Light", "Camera")):
            bpy.data.objects.remove(obj, do_unlink=True)


def setup_camera(args):
    """Create orthographic camera at top-down angle."""
    cam_data = bpy.data.cameras.new("SpriteCamera")
    cam_data.type = "ORTHO"
    cam_data.ortho_scale = 2.0  # Adjust to frame the character

    cam_obj = bpy.data.objects.new("SpriteCamera", cam_data)
    bpy.context.scene.collection.objects.link(cam_obj)
    bpy.context.scene.camera = cam_obj

    # Position: elevated, looking down at angle
    angle_rad = math.radians(args.camera_angle)
    dist = args.camera_distance
    cam_obj.location = (0, -dist * math.cos(angle_rad), dist * math.sin(angle_rad))
    cam_obj.rotation_euler = Euler((angle_rad, 0, 0), "XYZ")

    return cam_obj


def setup_lighting():
    """Simple 3-point lighting for clear silhouettes."""
    # Key light (sun)
    key_data = bpy.data.lights.new("KeyLight", "SUN")
    key_data.energy = 3.0
    key_obj = bpy.data.objects.new("KeyLight", key_data)
    key_obj.rotation_euler = Euler((math.radians(45), 0, math.radians(30)), "XYZ")
    bpy.context.scene.collection.objects.link(key_obj)

    # Fill light (dimmer, opposite side)
    fill_data = bpy.data.lights.new("FillLight", "SUN")
    fill_data.energy = 1.0
    fill_obj = bpy.data.objects.new("FillLight", fill_data)
    fill_obj.rotation_euler = Euler((math.radians(60), 0, math.radians(-45)), "XYZ")
    bpy.context.scene.collection.objects.link(fill_obj)

    # Rim light (from behind, for silhouette pop)
    rim_data = bpy.data.lights.new("RimLight", "SUN")
    rim_data.energy = 1.5
    rim_obj = bpy.data.objects.new("RimLight", rim_data)
    rim_obj.rotation_euler = Euler((math.radians(30), 0, math.radians(180)), "XYZ")
    bpy.context.scene.collection.objects.link(rim_obj)


# ---------------------------------------------------------------------------
# Armature and animation helpers
# ---------------------------------------------------------------------------

def find_armature():
    """Find the first armature in the scene."""
    for obj in bpy.data.objects:
        if obj.type == "ARMATURE":
            return obj
    return None


def find_action(search_terms):
    """Find an animation action matching any of the search terms (case-insensitive)."""
    for action in bpy.data.actions:
        name_lower = action.name.lower()
        for term in search_terms:
            if term.lower() in name_lower:
                return action
    return None


def get_action_frame_range(action):
    """Get the start and end frame of an action."""
    return int(action.frame_range[0]), int(action.frame_range[1])


def sample_frames(start, end, count):
    """Pick `count` evenly-spaced frames from [start, end]."""
    if count <= 1:
        return [start]
    step = (end - start) / (count - 1)
    return [int(start + i * step) for i in range(count)]


# ---------------------------------------------------------------------------
# Rendering
# ---------------------------------------------------------------------------

def render_frame(output_path):
    """Render the current frame to a file."""
    bpy.context.scene.render.filepath = output_path
    bpy.ops.render.render(write_still=True)


def render_animation_frames(armature, action, frame_count, rotation_z, tmp_dir, frame_prefix):
    """Render `frame_count` frames of an action at a given Z rotation.

    Returns list of rendered file paths.
    """
    # Set active action
    if armature.animation_data is None:
        armature.animation_data_create()
    armature.animation_data.action = action

    start, end = get_action_frame_range(action)
    frames = sample_frames(start, end, frame_count)

    # Apply rotation to the armature
    original_rotation = armature.rotation_euler.z
    armature.rotation_euler.z = rotation_z

    rendered = []
    for i, frame in enumerate(frames):
        bpy.context.scene.frame_set(frame)
        path = os.path.join(tmp_dir, f"{frame_prefix}_{i:03d}.png")
        render_frame(path)
        rendered.append(path)

    # Restore rotation
    armature.rotation_euler.z = original_rotation
    return rendered


# ---------------------------------------------------------------------------
# Sprite sheet compositing
# ---------------------------------------------------------------------------

def composite_sprite_sheet(frame_paths, columns, frame_size, output_path):
    """Combine individual frame PNGs into a single sprite sheet.

    Uses Blender's compositor/image API to avoid external dependencies.
    """
    import struct

    rows = math.ceil(len(frame_paths) / columns)
    sheet_w = columns * frame_size
    sheet_h = rows * frame_size

    # Create output image (RGBA float buffer)
    sheet_name = "SpriteSheet"
    if sheet_name in bpy.data.images:
        bpy.data.images.remove(bpy.data.images[sheet_name])
    sheet = bpy.data.images.new(sheet_name, sheet_w, sheet_h, alpha=True)

    # Initialize to transparent
    pixels = [0.0] * (sheet_w * sheet_h * 4)

    for idx, fpath in enumerate(frame_paths):
        if not os.path.exists(fpath):
            continue

        # Load frame image
        frame_name = f"_frame_{idx}"
        if frame_name in bpy.data.images:
            bpy.data.images.remove(bpy.data.images[frame_name])
        frame_img = bpy.data.images.load(fpath, check_existing=False)
        frame_img.name = frame_name

        fw = frame_img.size[0]
        fh = frame_img.size[1]
        frame_pixels = list(frame_img.pixels)

        # Destination position in sheet
        col = idx % columns
        row = idx // columns
        # Blender images are bottom-up, but we want top-left origin for game use.
        # We'll flip the entire sheet at the end, so place rows bottom-up for now.
        dest_y = (rows - 1 - row) * frame_size
        dest_x = col * frame_size

        # Copy pixels
        for py in range(min(fh, frame_size)):
            for px in range(min(fw, frame_size)):
                src_offset = (py * fw + px) * 4
                dst_offset = ((dest_y + py) * sheet_w + dest_x + px) * 4
                pixels[dst_offset:dst_offset + 4] = frame_pixels[src_offset:src_offset + 4]

        # Clean up frame image
        bpy.data.images.remove(frame_img)

    sheet.pixels = pixels
    sheet.filepath_raw = output_path
    sheet.file_format = "PNG"
    sheet.save()

    # Clean up
    bpy.data.images.remove(sheet)


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    args = parse_args()
    output_path = os.path.abspath(args.output)
    output_dir = os.path.dirname(output_path)
    os.makedirs(output_dir, exist_ok=True)

    # Temp directory for individual frames
    tmp_dir = os.path.join(output_dir, "_tmp_frames")
    os.makedirs(tmp_dir, exist_ok=True)

    setup_scene(args)
    setup_camera(args)
    setup_lighting()

    armature = find_armature()
    if armature is None:
        print("ERROR: No armature found in scene")
        sys.exit(1)

    # Also parent all mesh children to move with armature rotation
    # (they should already be children, but ensure it)

    all_frames = []
    metadata = {
        "frame_size": [args.size, args.size],
        "columns": args.columns,
        "animations": {},
    }

    frame_index = 0

    for anim_cfg in ANIMATION_CONFIG:
        action = find_action(anim_cfg["search"])
        if action is None:
            print(f"WARNING: No action found for '{anim_cfg['name']}' "
                  f"(searched: {anim_cfg['search']}). Skipping.")
            continue

        print(f"Rendering '{anim_cfg['name']}' using action '{action.name}' "
              f"({anim_cfg['frames']} frames, directional={anim_cfg['directional']})")

        if anim_cfg["directional"]:
            for dir_name, rot_z in DIRECTION_ROTATIONS.items():
                anim_key = f"{anim_cfg['name']}_{dir_name}"
                frames = render_animation_frames(
                    armature, action, anim_cfg["frames"],
                    rot_z, tmp_dir, f"{anim_cfg['name']}_{dir_name}"
                )
                all_frames.extend(frames)
                metadata["animations"][anim_key] = {
                    "start": frame_index,
                    "count": len(frames),
                }
                frame_index += len(frames)
        else:
            frames = render_animation_frames(
                armature, action, anim_cfg["frames"],
                0.0, tmp_dir, anim_cfg["name"]
            )
            all_frames.extend(frames)
            metadata["animations"][anim_cfg["name"]] = {
                "start": frame_index,
                "count": len(frames),
            }
            frame_index += len(frames)

    if not all_frames:
        print("ERROR: No frames were rendered")
        sys.exit(1)

    # Update metadata with actual row count
    rows = math.ceil(len(all_frames) / args.columns)
    metadata["rows"] = rows

    print(f"Compositing {len(all_frames)} frames into {args.columns}x{rows} sheet...")
    composite_sprite_sheet(all_frames, args.columns, args.size, output_path)

    # Write JSON metadata
    json_path = os.path.splitext(output_path)[0] + ".json"
    with open(json_path, "w") as f:
        json.dump(metadata, f, indent=2)
    print(f"Written: {output_path}")
    print(f"Written: {json_path}")

    # Clean up temp frames
    import shutil
    shutil.rmtree(tmp_dir, ignore_errors=True)


if __name__ == "__main__":
    main()
