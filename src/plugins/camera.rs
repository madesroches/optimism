use bevy::prelude::*;
use micromegas_tracing::prelude::*;

use super::maze::MazeMap;
use super::maze::TILE_SIZE;
use super::telemetry::GameSet;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_camera);
        app.add_systems(Update, fit_camera_to_maze.in_set(GameSet::Presentation));
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

/// Scale the camera to fit the maze with some padding.
#[span_fn]
fn fit_camera_to_maze(
    maze: Option<Res<MazeMap>>,
    windows: Query<&Window>,
    mut cameras: Query<&mut Projection, With<Camera2d>>,
) {
    let Some(maze) = maze else { return };
    let Ok(window) = windows.single() else {
        return;
    };
    let Ok(mut projection) = cameras.single_mut() else {
        return;
    };

    let maze_width = maze.width as f32 * TILE_SIZE;
    let maze_height = maze.height as f32 * TILE_SIZE;

    let padding = TILE_SIZE * 2.0;
    let total_width = maze_width + padding;
    let total_height = maze_height + padding;

    let scale_x = total_width / window.width();
    let scale_y = total_height / window.height();

    if let Projection::Orthographic(ref mut ortho) = *projection {
        ortho.scale = scale_x.max(scale_y);
    }
}
