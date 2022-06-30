use bevy::{
    prelude::*,
    utils::{HashMap, HashSet}, render::view::NoFrustumCulling,
};

use crate::{
    fly_by_camera::FlyByCamera,
    world::{
        pipeline::{
            genesis::{EvtChunkLoaded, EvtChunkUnloaded},
            ChunkMaterial, ChunkMaterialHandle,
        },
        query,
        storage::{chunk, landscape},
    },
};

use super::{
    genesis::{BatchChunkCmdRes, WorldRes},
    ChunkBundle, ChunkEntityMap, ChunkLocal, EvtChunkMeshDirty, EvtChunkUpdated,
};

pub(super) struct LandscapingPlugin;

impl Plugin for LandscapingPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<EvtChunkMeshDirty>()
            .add_plugin(MaterialPlugin::<ChunkMaterial>::default())
            .add_startup_system_to_stage(super::PipelineStartup::Landscaping, setup_resources)
            .add_system_set_to_stage(
                super::Pipeline::Landscaping,
                SystemSet::new()
                    .with_system(despawn_chunks_system.label("despawn"))
                    .with_system(spawn_chunks_system.label("spawn").after("despawn"))
                    .with_system(update_chunks_system.after("spawn"))
                    .with_system(update_landscape_system.label("update")),
            );
    }
}

#[derive(Default)]
pub struct LandscapeConfig {
    pub paused: bool,
}

fn setup_resources(mut commands: Commands, mut materials: ResMut<Assets<ChunkMaterial>>) {
    trace_system_run!();
    let material = materials.add(ChunkMaterial);

    commands.insert_resource(ChunkMaterialHandle(material));
    commands.insert_resource(ChunkEntityMap(HashMap::default()));
    commands.insert_resource(LandscapeConfig { paused: false })
}

#[derive(Default)]
struct UpdateLandscapeMeta {
    last_pos: IVec3,
    next_sync: f32,
}

fn update_landscape_system(
    time: Res<Time>,
    entity_map: ResMut<ChunkEntityMap>,
    config: Res<LandscapeConfig>,
    world_res: Res<WorldRes>,
    mut meta: Local<UpdateLandscapeMeta>,
    mut batch: ResMut<BatchChunkCmdRes>,
    q: Query<&Transform, With<FlyByCamera>>,
) {
    let mut _perf = perf_fn!();

    if config.paused || !world_res.is_ready() {
        return;
    }

    let center = match q.get_single() {
        Ok(t) => chunk::to_local(t.translation),
        Err(_) => return,
    };

    meta.next_sync -= time.delta_seconds();

    if center != meta.last_pos || meta.next_sync < 0.0 {
        perf_scope!(_perf);
        meta.next_sync = 1.0;
        meta.last_pos = center;

        debug!("Updating landscape to center {}", center);

        let begin = center + IVec3::splat(landscape::BEGIN);
        let end = center + IVec3::splat(landscape::END);

        let visible_locals = query::range(begin, end).collect::<HashSet<_>>();
        let existing_locals = entity_map.0.keys().copied().collect::<HashSet<_>>();

        visible_locals
            .iter()
            .filter(|&i| !existing_locals.contains(i))
            .for_each(|v| batch.load(*v));

        existing_locals
            .iter()
            .filter(|&i| !visible_locals.contains(i))
            .for_each(|v| batch.unload(*v));
    }
}

fn spawn_chunks_system(
    mut commands: Commands,
    mut entity_map: ResMut<ChunkEntityMap>,
    chunk_pipeline: Res<ChunkMaterialHandle>,
    mut reader: EventReader<EvtChunkLoaded>,
    mut writer: EventWriter<EvtChunkMeshDirty>,
) {
    let mut _perf = perf_fn!();
    for EvtChunkLoaded(local) in reader.iter() {
        trace_system_run!(local);
        perf_scope!(_perf);

        let entity = commands
            .spawn_bundle(ChunkBundle {
                local: ChunkLocal(*local),
                mesh_bundle: MaterialMeshBundle {
                    material: chunk_pipeline.0.clone(),
                    transform: Transform::from_translation(chunk::to_world(*local)),
                    ..Default::default()
                },
            })
            .insert(NoFrustumCulling)
            .id();
        entity_map.0.insert(*local, entity);
        writer.send(EvtChunkMeshDirty(*local));
    }
}

fn despawn_chunks_system(
    mut commands: Commands,
    mut entity_map: ResMut<ChunkEntityMap>,
    mut reader: EventReader<EvtChunkUnloaded>,
) {
    let mut _perf = perf_fn!();

    for EvtChunkUnloaded(local) in reader.iter() {
        trace_system_run!(local);
        perf_scope!(_perf);

        if let Some(entity) = entity_map.0.remove(local) {
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn update_chunks_system(
    mut reader: EventReader<EvtChunkUpdated>,
    mut writer: EventWriter<EvtChunkMeshDirty>,
    entity_map: ResMut<ChunkEntityMap>,
) {
    let mut _perf = perf_fn!();

    for EvtChunkUpdated(chunk_local) in reader.iter() {
        if entity_map.0.get(chunk_local).is_some() {
            trace_system_run!(chunk_local);
            perf_scope!(_perf);
            writer.send(EvtChunkMeshDirty(*chunk_local));
        }
    }
}

#[cfg(test)]
mod test {
    use bevy::{ecs::event::Events, prelude::*, utils::HashMap};

    use crate::world::pipeline::{
        genesis::EvtChunkUnloaded, ChunkBundle, ChunkLocal, ChunkMaterialHandle, EvtChunkMeshDirty,
        EvtChunkUpdated,
    };

    use super::{ChunkEntityMap, EvtChunkLoaded};

    #[test]
    fn spawn_chunks_system() {
        // Arrange
        let mut added_events = Events::<EvtChunkLoaded>::default();
        added_events.send(EvtChunkLoaded(IVec3::ONE));

        let mut world = World::default();
        world.insert_resource(ChunkEntityMap(HashMap::default()));
        world.insert_resource(added_events);
        world.insert_resource(Events::<EvtChunkMeshDirty>::default());
        world.insert_resource(ChunkMaterialHandle(Handle::default()));

        let mut stage = SystemStage::parallel();
        stage.add_system(super::spawn_chunks_system);

        // Act
        stage.run(&mut world);

        // Assert
        assert_eq!(
            world
                .get_resource::<Events<EvtChunkMeshDirty>>()
                .unwrap()
                .iter_current_update_events()
                .next()
                .unwrap()
                .0,
            IVec3::ONE
        );

        assert_eq!(world.query::<&ChunkLocal>().iter(&world).len(), 1);
    }

    #[test]
    fn despawn_chunks_system() {
        // Arrange
        let mut added_events = Events::<EvtChunkUnloaded>::default();
        added_events.send(EvtChunkUnloaded(IVec3::ONE));

        let mut world = World::default();
        world.insert_resource(added_events);
        world.insert_resource(Events::<super::EvtChunkMeshDirty>::default());

        let entity = world
            .spawn()
            .insert_bundle(ChunkBundle {
                local: ChunkLocal(IVec3::ONE),
                ..Default::default()
            })
            .id();

        let mut entity_map = ChunkEntityMap(HashMap::default());
        entity_map.0.insert(IVec3::ONE, entity);
        world.insert_resource(entity_map);

        let mut stage = SystemStage::parallel();
        stage.add_system(super::despawn_chunks_system);

        // Act
        stage.run(&mut world);

        // Assert
        assert_eq!(world.query::<&ChunkLocal>().iter(&world).len(), 0);
        assert!(world.get_resource::<ChunkEntityMap>().unwrap().0.is_empty());
    }

    #[test]
    fn update_chunks_system() {
        // Arrange
        let mut added_events = Events::<EvtChunkUpdated>::default();
        added_events.send(EvtChunkUpdated((1, 2, 3).into()));

        let mut world = World::default();
        world.insert_resource(added_events);
        world.insert_resource(Events::<super::EvtChunkMeshDirty>::default());

        let mut entity_map = ChunkEntityMap(HashMap::default());
        entity_map.0.insert(
            (1, 2, 3).into(),
            world.spawn().insert_bundle(ChunkBundle::default()).id(),
        );
        world.insert_resource(entity_map);

        let mut stage = SystemStage::parallel();
        stage.add_system(super::update_chunks_system);

        // Act
        stage.run(&mut world);

        // Assert
        assert_eq!(
            world
                .get_resource::<Events<EvtChunkMeshDirty>>()
                .unwrap()
                .iter_current_update_events()
                .next()
                .unwrap()
                .0,
            (1, 2, 3).into()
        );
    }
}
