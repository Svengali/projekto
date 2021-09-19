use bevy::{
    core::FixedTimestep,
    prelude::*,
    render::{
        pipeline::{PipelineDescriptor, RenderPipeline},
        shader::ShaderStages,
    },
    utils::{HashMap, HashSet},
};

use crate::{
    fly_by_camera::FlyByCamera,
    world::{
        pipeline::genesis::{CmdChunkUnload, EvtChunkLoaded, EvtChunkUnloaded},
        query,
        storage::{chunk, landscape},
    },
};

use super::{
    genesis::CmdChunkLoad, ChunkBuildingBundle, ChunkBundle, ChunkEntityMap, ChunkLocal,
    ChunkPipeline, EvtChunkDirty, EvtChunkUpdated,
};

pub(super) struct LandscapingPlugin;

impl Plugin for LandscapingPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<EvtChunkDirty>()
            .add_startup_system_to_stage(super::PipelineStartup::Landscaping, setup_resources)
            // .add_startup_system_to_stage(super::PipelineStartup::Landscaping, setup_landscape)
            .add_system_set_to_stage(
                super::Pipeline::Landscaping,
                SystemSet::new()
                    .with_system(despawn_chunks_system.label("despawn"))
                    .with_system(spawn_chunks_system.label("spawn").after("despawn"))
                    .with_system(update_chunks_system.after("spawn"))
                    .with_system(
                        update_landscape_system
                            .label("update")
                            .with_run_criteria(FixedTimestep::step(0.1)),
                    ),
            );
    }
}

fn setup_resources(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut pipelines: ResMut<Assets<PipelineDescriptor>>,
) {
    trace_system_run!();

    let pipeline_handle = pipelines.add(PipelineDescriptor {
        // primitive: PrimitiveState {
        //     topology: PrimitiveTopology::TriangleList,
        //     strip_index_format: None,
        //     front_face: FrontFace::Ccw,
        //     cull_mode: Some(Face::Back),
        //     polygon_mode: PolygonMode::Fill,
        //     clamp_depth: false,
        //     conservative: false,
        // },
        ..PipelineDescriptor::default_config(ShaderStages {
            vertex: asset_server.load("shaders/voxel.vert"),
            fragment: Some(asset_server.load("shaders/voxel.frag")),
        })
    });

    commands.insert_resource(ChunkPipeline(pipeline_handle));
    commands.insert_resource(ChunkEntityMap(HashMap::default()));
}

fn update_landscape_system(
    entity_map: ResMut<ChunkEntityMap>,
    mut add_writer: EventWriter<CmdChunkLoad>,
    mut remove_writer: EventWriter<CmdChunkUnload>,
    mut req_add: Local<HashSet<IVec3>>,
    mut added: EventReader<EvtChunkLoaded>,
    q: Query<&Transform, With<FlyByCamera>>,
    mut last_pos: Local<IVec3>,
) {
    let mut _perf = perf_fn!();

    let center = match q.single() {
        Ok(t) => chunk::to_local(t.translation),
        Err(_) => return,
    };

    if center == *last_pos {
        *last_pos = center;
    }

    perf_scope!(_perf);

    let begin = center + IVec3::splat(landscape::BEGIN);
    let end = center + IVec3::splat(landscape::END);

    let visible_locals = query::range(begin, end).collect::<Vec<_>>();
    let existing_locals = entity_map.0.keys().map(|k| *k).collect::<Vec<_>>();

    let (to_add, to_remove) = disjoin(&visible_locals, &existing_locals);

    for EvtChunkLoaded(local) in added.iter() {
        req_add.remove(local);
    }

    for local in to_add {
        if req_add.contains(local) {
            continue;
        }

        add_writer.send(CmdChunkLoad(*local));
        req_add.insert(*local);
    }

    for local in to_remove {
        remove_writer.send(CmdChunkUnload(*local));
    }
}

fn disjoin<'a>(
    set_a: &'a [IVec3],
    set_b: &'a [IVec3],
) -> (
    impl Iterator<Item = &'a IVec3>,
    impl Iterator<Item = &'a IVec3>,
) {
    (
        set_a.iter().filter(move |v| !set_b.contains(v)),
        set_b.iter().filter(move |v| !set_a.contains(v)),
    )
}

fn spawn_chunks_system(
    mut commands: Commands,
    mut entity_map: ResMut<ChunkEntityMap>,
    chunk_pipeline: Res<ChunkPipeline>,
    mut reader: EventReader<EvtChunkLoaded>,
    mut writer: EventWriter<EvtChunkDirty>,
) {
    let mut _perf = perf_fn!();
    for EvtChunkLoaded(local) in reader.iter() {
        trace_system_run!(local);
        perf_scope!(_perf);

        let entity = commands
            .spawn_bundle(ChunkBundle {
                local: ChunkLocal(*local),
                mesh_bundle: MeshBundle {
                    render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                        chunk_pipeline.0.clone(),
                    )]),
                    transform: Transform::from_translation(chunk::to_world(*local)),
                    ..Default::default()
                },
                ..Default::default()
            })
            .id();
        entity_map.0.insert(*local, entity);
        writer.send(EvtChunkDirty(*local));
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
    mut commands: Commands,
    mut reader: EventReader<EvtChunkUpdated>,
    mut writer: EventWriter<EvtChunkDirty>,
    entity_map: ResMut<ChunkEntityMap>,
) {
    let mut _perf = perf_fn!();

    for EvtChunkUpdated(chunk_local) in reader.iter() {
        if let Some(&entity) = entity_map.0.get(chunk_local) {
            trace_system_run!(chunk_local);
            perf_scope!(_perf);

            commands
                .entity(entity)
                .insert_bundle(ChunkBuildingBundle::default());
            writer.send(EvtChunkDirty(*chunk_local));
        }
    }
}

#[cfg(test)]
mod test {
    use std::vec;

    use bevy::{app::Events, prelude::*, utils::HashMap};

    use crate::world::pipeline::{
        genesis::EvtChunkUnloaded, ChunkBundle, ChunkLocal, ChunkPipeline, EvtChunkDirty,
    };

    use super::{ChunkEntityMap, EvtChunkLoaded, EvtChunkUpdated};

    #[test]
    fn disjoint() {
        let a = vec![
            (0, 0, 0).into(),
            (1, 1, 1).into(),
            (2, 2, 2).into(),
            (3, 3, 3).into(),
        ];
        let b = vec![
            (0, 0, 0).into(),
            (1, 1, 1).into(),
            (2, 2, 3).into(),
            (3, 3, 4).into(),
        ];

        let (d_a, d_b) = super::disjoin(&a, &b);

        let disjoint_a = d_a.map(|v| *v).collect::<Vec<_>>();
        let disjoint_b = d_b.map(|v| *v).collect::<Vec<_>>();

        assert_eq!(disjoint_a, vec![(2, 2, 2).into(), (3, 3, 3).into()]);
        assert_eq!(disjoint_b, vec![(2, 2, 3).into(), (3, 3, 4).into()]);
    }

    #[test]
    fn spawn_chunks_system() {
        // Arrange
        let mut added_events = Events::<EvtChunkLoaded>::default();
        added_events.send(EvtChunkLoaded(IVec3::ONE));

        let mut world = World::default();
        world.insert_resource(ChunkEntityMap(HashMap::default()));
        world.insert_resource(added_events);
        world.insert_resource(Events::<EvtChunkDirty>::default());
        world.insert_resource(ChunkPipeline(Handle::default()));

        let mut stage = SystemStage::parallel();
        stage.add_system(super::spawn_chunks_system);

        // Act
        stage.run(&mut world);

        // Assert
        assert_eq!(
            world
                .get_resource::<Events<EvtChunkDirty>>()
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
        world.insert_resource(Events::<super::EvtChunkDirty>::default());

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
        world.insert_resource(Events::<super::EvtChunkDirty>::default());

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
                .get_resource::<Events<EvtChunkDirty>>()
                .unwrap()
                .iter_current_update_events()
                .next()
                .unwrap()
                .0,
            (1, 2, 3).into()
        );
    }
}