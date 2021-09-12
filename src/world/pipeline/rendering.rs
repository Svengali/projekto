use bevy::{
    prelude::*,
    render::{mesh::Indices, pipeline::PrimitiveTopology},
};

use crate::world::{
    mesh,
    storage::{
        self, chunk,
        voxel::{self, VoxelVertex},
        VoxWorld,
    },
};

use super::{
    ChunkBuildingBundle, ChunkEntityMap, ChunkFaces, ChunkFacesOcclusion, ChunkVertices,
    EvtChunkDirty, Pipeline,
};

pub struct RenderingPlugin;

impl Plugin for RenderingPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_to_stage(
            Pipeline::Rendering,
            faces_occlusion_system.label("faces_occlusion"),
        )
        .add_system_to_stage(
            Pipeline::Rendering,
            faces_merging_system
                .label("faces_merging")
                .after("faces_occlusion"),
        )
        .add_system_to_stage(
            Pipeline::Rendering,
            vertices_computation_system
                .label("vertices")
                .after("faces_merging"),
        )
        .add_system_to_stage(
            Pipeline::Rendering,
            mesh_generation_system
                .label("mesh_generation")
                .after("vertices"),
        )
        .add_system_to_stage(
            Pipeline::Rendering,
            clean_up_system.after("mesh_generation"),
        );
    }
}

fn faces_occlusion_system(
    world: Res<storage::VoxWorld>,
    entity_map: Res<ChunkEntityMap>,
    mut reader: EventReader<EvtChunkDirty>,
    mut q: Query<&mut ChunkFacesOcclusion>,
) {
    for EvtChunkDirty(local) in reader.iter() {
        let chunk = match world.get(*local) {
            None => {
                warn!(
                    "Skipping faces occlusion since chunk {} wasn't found on world",
                    *local
                );
                continue;
            }
            Some(c) => c,
        };

        let entity = match entity_map.0.get(local) {
            None => {
                warn!(
                    "Skipping faces occlusion since chunk {} wasn't found on entity map",
                    *local
                );
                continue;
            }
            Some(&e) => e,
        };

        let mut faces_occlusion = match q.get_mut(entity) {
            Err(e) => {
                warn!(
                    "Skipping faces occlusion for chunk {}. Error: {}",
                    *local, e
                );
                continue;
            }
            Ok(f) => f,
        };

        trace!("Processing faces occlusion of chunk entity {}", *local);

        faces_occlusion.0.fill(voxel::FacesOcclusion::default());

        for voxel in chunk::voxels() {
            let voxel_faces = &mut faces_occlusion.0[chunk::to_index(voxel)];

            if chunk.get_kind(voxel) == 0 {
                voxel_faces.fill(true);
                continue;
            }

            for side in voxel::SIDES {
                let dir = voxel::get_side_dir(side);
                let neighbor_pos = voxel + dir;

                if !chunk::is_within_bounds(neighbor_pos) {
                    // TODO: Check neighborhood
                    continue;
                }

                if chunk.get_kind(neighbor_pos) == 1 {
                    voxel_faces[side as usize] = true;
                }
            }
        }
    }
}

fn vertices_computation_system(
    entity_map: Res<ChunkEntityMap>,
    mut reader: EventReader<EvtChunkDirty>,
    mut q: Query<(&ChunkFaces, &mut ChunkVertices)>,
) {
    for EvtChunkDirty(local) in reader.iter() {
        let entity = match entity_map.0.get(local) {
            None => {
                warn!(
                    "Skipping vertices computation since chunk {} wasn't found on entity map",
                    *local
                );
                continue;
            }
            Some(&e) => e,
        };

        let (faces, mut vertices) = match q.get_mut(entity) {
            Err(e) => {
                warn!(
                    "Skipping vertices computation for chunk {}. Error: {}",
                    *local, e
                );
                continue;
            }
            Ok(f) => f,
        };

        trace!("Processing vertices computation of chunk entity {}", *local);

        vertices.0.clear();

        for face in faces.0.iter() {
            let normal = voxel::get_side_normal(face.side);

            for (i, v) in face.vertices.iter().enumerate() {
                let base_vertex_idx = mesh::VERTICES_INDICES[face.side as usize][i];
                let base_vertex: Vec3 = mesh::VERTICES[base_vertex_idx].into();
                vertices.0.push(VoxelVertex {
                    position: base_vertex + v.as_f32(),
                    normal,
                })
            }
        }
    }
}

fn mesh_generation_system(
    mut commands: Commands,
    entity_map: Res<ChunkEntityMap>,
    mut reader: EventReader<EvtChunkDirty>,
    mut meshes: ResMut<Assets<Mesh>>,
    query: Query<&ChunkVertices>,
) {
    for EvtChunkDirty(local) in reader.iter() {
        let entity = match entity_map.0.get(local) {
            None => {
                warn!(
                    "Skipping mesh generation since chunk {} wasn't found on entity map",
                    *local
                );
                continue;
            }
            Some(&e) => e,
        };

        let vertices = match query.get(entity) {
            Err(e) => {
                warn!(
                    "Skipping vertices computation for chunk {}. Error: {}",
                    *local, e
                );
                continue;
            }
            Ok(v) => &v.0,
        };

        trace!("Processing mesh generation of chunk entity {}", *local);

        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);

        let mut positions: Vec<[f32; 3]> = vec![];
        let mut normals: Vec<[f32; 3]> = vec![];

        let vertex_count = vertices.len();

        for vertex in vertices {
            positions.push([vertex.position.x, vertex.position.y, vertex.position.z]);
            normals.push([vertex.normal.x, vertex.normal.y, vertex.normal.z]);
        }

        mesh.set_indices(Some(Indices::U32(mesh::compute_indices(vertex_count))));
        mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.set_attribute(Mesh::ATTRIBUTE_NORMAL, normals);

        commands.entity(entity).insert(meshes.add(mesh));
    }
}

fn clean_up_system(
    mut commands: Commands,
    mut reader: EventReader<EvtChunkDirty>,
    entity_map: Res<ChunkEntityMap>,
) {
    for EvtChunkDirty(local) in reader.iter() {
        let entity = match entity_map.0.get(local) {
            None => {
                warn!(
                    "Skipping clean up since chunk {} wasn't found on entity map",
                    *local
                );
                continue;
            }
            Some(&e) => e,
        };

        trace!("Clearing up chunk entity {}", *local);

        commands
            .entity(entity)
            .remove_bundle::<ChunkBuildingBundle>();
    }
}

fn faces_merging_system(
    mut reader: EventReader<EvtChunkDirty>,
    vox_world: Res<VoxWorld>,
    entity_map: Res<ChunkEntityMap>,
    mut query: Query<(&mut ChunkFaces, &ChunkFacesOcclusion)>,
) {
    for EvtChunkDirty(local) in reader.iter() {
        let entity = match entity_map.0.get(local) {
            Some(&e) => e,
            None => {
                warn!(
                    "Skipping faces merging since chunk {} wasn't found on entity map",
                    *local
                );
                continue;
            }
        };

        let (mut faces, occlusion) = match query.get_mut(entity) {
            Ok(v) => v,
            Err(e) => {
                warn!("Skipping faces merging for chunk {}. Error: {}", *local, e);
                continue;
            }
        };

        let chunk = match vox_world.get(*local) {
            None => {
                warn!(
                    "Skipping faces occlusion since chunk {} wasn't found on world",
                    *local
                );
                continue;
            }
            Some(c) => c,
        };

        let merged_faces = mesh::merge_faces(&occlusion.0, chunk);
        faces.0 = merged_faces;
    }
}

#[cfg(test)]
mod test {
    use bevy::{app::Events, utils::HashMap};

    use crate::world::pipeline::ChunkBuildingBundle;

    use super::*;

    #[test]
    fn faces_occlusion_system_occlude_empty_voxel() {
        // Arrange
        let local = (3, 2, 1).into();

        let mut events = Events::<EvtChunkDirty>::default();
        events.send(EvtChunkDirty(local));

        let mut voxel_world = storage::VoxWorld::default();
        voxel_world.add(local);

        let mut world = World::default();
        world.insert_resource(voxel_world);
        world.insert_resource(events);

        let mut entity_map = ChunkEntityMap(HashMap::default());

        entity_map.0.insert(
            local,
            world
                .spawn()
                .insert_bundle(ChunkBuildingBundle::default())
                .id(),
        );

        world.insert_resource(entity_map);

        let mut stage = SystemStage::parallel();
        stage.add_system(super::faces_occlusion_system);

        // Act
        stage.run(&mut world);

        // Assert
        let faces_occlusion = world
            .query::<&ChunkFacesOcclusion>()
            .iter(&world)
            .next()
            .unwrap();

        assert!(
            faces_occlusion
                .0
                .iter()
                .all(|a| a.iter().all(|b| *b == true)),
            "A chunk full of empty-kind voxels should be fully occluded"
        );
    }

    #[test]
    fn faces_occlusion_system() {
        // Arrange
        let local = (3, 2, 1).into();

        let mut events = Events::<EvtChunkDirty>::default();
        events.send(EvtChunkDirty(local));

        let mut voxel_world = storage::VoxWorld::default();
        voxel_world.add(local);

        let chunk = voxel_world.get_mut(local).unwrap();
        // Top-Bottom occlusion
        chunk.set_kind((1, 1, 1).into(), 1);
        chunk.set_kind((1, 2, 1).into(), 1);

        // Full occluded voxel at (10, 10, 10)
        chunk.set_kind((10, 10, 10).into(), 1);
        chunk.set_kind((9, 10, 10).into(), 1);
        chunk.set_kind((11, 10, 10).into(), 1);
        chunk.set_kind((10, 9, 10).into(), 1);
        chunk.set_kind((10, 11, 10).into(), 1);
        chunk.set_kind((10, 10, 9).into(), 1);
        chunk.set_kind((10, 10, 11).into(), 1);

        let mut world = World::default();
        world.insert_resource(voxel_world);
        world.insert_resource(events);

        let mut entity_map = ChunkEntityMap(HashMap::default());

        entity_map.0.insert(
            local,
            world
                .spawn()
                .insert_bundle(ChunkBuildingBundle::default())
                .id(),
        );

        world.insert_resource(entity_map);

        let mut stage = SystemStage::parallel();
        stage.add_system(super::faces_occlusion_system);

        // Act
        stage.run(&mut world);

        // Assert
        let faces_occlusion = world
            .query::<&ChunkFacesOcclusion>()
            .iter(&world)
            .next()
            .unwrap();

        let faces = faces_occlusion.0[chunk::to_index((1, 2, 1).into())];

        assert_eq!(
            faces,
            [false, false, false, true, false, false],
            "Only down face should be occluded by the bottom voxel"
        );

        let faces = faces_occlusion.0[chunk::to_index((1, 1, 1).into())];

        assert_eq!(
            faces,
            [false, false, true, false, false, false],
            "Only down face should be occluded by the bottom voxel"
        );

        let faces = faces_occlusion.0[chunk::to_index((10, 10, 10).into())];

        assert_eq!(
            faces,
            [true; voxel::SIDE_COUNT],
            "Voxel fully surrounded by another non-empty voxels should be fully occluded"
        );
    }

    // #[test]
    // fn vertices_computation_system() {
    //     // Arrange
    //     let local = (1, 2, 3).into();

    //     let mut events = Events::<EvtChunkDirty>::default();
    //     events.send(EvtChunkDirty(local));

    //     let mut world = World::default();
    //     world.insert_resource(events);

    //     let mut entity_map = ChunkEntityMap(HashMap::default());

    //     let mut faces_occlusion =
    //         ChunkFacesOcclusion([voxel::FacesOcclusion::default(); chunk::BUFFER_SIZE]);

    //     faces_occlusion.0.fill([true; voxel::SIDE_COUNT]);

    //     let full_visible = (1, 1, 1).into();
    //     faces_occlusion.0[chunk::to_index(full_visible)] = [false; voxel::SIDE_COUNT];

    //     let right_visible = (2, 1, 1).into();
    //     faces_occlusion.0[chunk::to_index(right_visible)] = [false, true, true, true, true, true];

    //     let entity = world
    //         .spawn()
    //         .insert_bundle(ChunkBuildingBundle {
    //             faces_occlusion,
    //             ..Default::default()
    //         })
    //         .id();

    //     entity_map.0.insert(local, entity);

    //     world.insert_resource(entity_map);

    //     let mut stage = SystemStage::parallel();
    //     stage.add_system(super::vertices_computation_system);

    //     // Act
    //     stage.run(&mut world);

    //     // Assert
    //     let vertices = world.query::<&ChunkVertices>().iter(&world).next().unwrap();

    //     for side in voxel::SIDES {
    //         if side == voxel::Side::Right {
    //             assert_eq!(
    //                 vertices.0[side as usize].len(),
    //                 8,
    //                 "There should 8 right-sided faces vertices"
    //             );
    //         } else {
    //             assert_eq!(
    //                 vertices.0[side as usize].len(),
    //                 4,
    //                 "There should 4 face vertices except for right-side"
    //             );
    //         }
    //     }
    // }

    // #[test]
    // fn mesh_generation_system() {
    //     // Arrange
    //     let local = (1, 2, 3).into();

    //     let mut events = Events::<EvtChunkDirty>::default();
    //     events.send(EvtChunkDirty(local));

    //     let mut world = World::default();
    //     world.insert_resource(events);

    //     let mut entity_map = ChunkEntityMap(HashMap::default());

    //     let asset_server = AssetServer::new(
    //         FileAssetIo::new(AssetServerSettings::default().asset_folder),
    //         TaskPool::new(),
    //     );

    //     // what now...

    //     world.insert_resource(asset_server);

    //     let entity = world
    //         .spawn()
    //         .insert_bundle(ChunkBuildingBundle {
    //             ..Default::default()
    //         })
    //         .id();

    //     entity_map.0.insert(local, entity);

    //     world.insert_resource(entity_map);

    //     let mut stage = SystemStage::parallel();
    //     stage.add_system(super::mesh_generation_system);

    //     // Act
    //     stage.run(&mut world);

    //     // Assert
    // }

    #[test]
    fn clean_up_system() {
        // Arrange
        let local = (1, 2, 3).into();

        let mut events = Events::<EvtChunkDirty>::default();
        events.send(EvtChunkDirty(local));

        let mut world = World::default();
        world.insert_resource(events);

        let mut entity_map = ChunkEntityMap(HashMap::default());

        let entity = world
            .spawn()
            .insert_bundle(ChunkBuildingBundle {
                ..Default::default()
            })
            .id();

        entity_map.0.insert(local, entity);

        world.insert_resource(entity_map);

        let mut stage = SystemStage::parallel();
        stage.add_system(super::clean_up_system);

        // Act
        stage.run(&mut world);

        // Assert
        assert!(world.get::<ChunkVertices>(entity).is_none());
    }
}
