use bevy::{prelude::*, wgpu::{WgpuFeature, WgpuFeatures, WgpuOptions}};

mod fly_by_camera;
use fly_by_camera::FlyByCameraPlugin;

mod world;
use world::WorldPlugin;

mod debug;
use debug::DebugPlugin;

fn main() {
    env_logger::init();

    App::new()
        .insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins)
        .add_plugin(DebugPlugin)
        .add_plugin(FlyByCameraPlugin)
        .add_plugin(WorldPlugin)
        .add_startup_system(setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // cube
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 0.1 })),
        material: materials.add(Color::rgb(0.3, 0.3, 0.3).into()),
        ..Default::default()
    });

    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..Default::default()
    });
}
