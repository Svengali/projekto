use bevy::{prelude::*, render::pipeline::PipelineDescriptor, utils::HashMap};

use self::{
    landscape::EntityManagingPlugin, render::RenderingPlugin, terraform::WorldManipulationPlugin,
};

mod genesis;
mod landscape;
mod render;
mod terraform;

pub use terraform::{
    CmdChunkAdd, CmdChunkRemove, CmdChunkUpdate, EvtChunkAdded, EvtChunkRemoved, EvtChunkUpdated,
};

use super::storage::{
    chunk::ChunkStorage,
    voxel::{self, VoxelFace, VoxelVertex},
};

#[derive(Debug, StageLabel, PartialEq, Eq, Hash, Clone, Copy)]
enum Pipeline {
    WorldManipulation,
    EntityManaging,
    Rendering,
}

#[derive(Debug, StageLabel, PartialEq, Eq, Hash, Clone, Copy)]
enum PipelineStartup {
    WorldManipulation,
    EntityManaging,
    Rendering,
}

pub struct PipelinePlugin;

impl Plugin for PipelinePlugin {
    fn build(&self, app: &mut App) {
        app.add_stage(Pipeline::WorldManipulation, SystemStage::parallel())
            .add_stage_after(
                Pipeline::WorldManipulation,
                Pipeline::EntityManaging,
                SystemStage::parallel(),
            )
            .add_stage_after(
                Pipeline::EntityManaging,
                Pipeline::Rendering,
                SystemStage::parallel(),
            )
            .add_startup_stage_after(
                StartupStage::Startup,
                PipelineStartup::WorldManipulation,
                SystemStage::parallel(),
            )
            .add_startup_stage_after(
                PipelineStartup::WorldManipulation,
                PipelineStartup::EntityManaging,
                SystemStage::parallel(),
            )
            .add_startup_stage_after(
                PipelineStartup::EntityManaging,
                PipelineStartup::Rendering,
                SystemStage::parallel(),
            );
        app.add_plugin(WorldManipulationPlugin)
            .add_plugin(EntityManagingPlugin)
            .add_plugin(RenderingPlugin);
    }
}

pub struct EvtChunkDirty(pub IVec3);

pub struct ChunkLocal(pub IVec3);

pub struct ChunkEntityMap(pub HashMap<IVec3, Entity>);

pub struct ChunkPipeline(Handle<PipelineDescriptor>);

pub type ChunkFacesOcclusion = ChunkStorage<voxel::FacesOcclusion>;

struct ChunkVertices(Vec<VoxelVertex>);
struct ChunkFaces(Vec<VoxelFace>);

#[derive(Bundle)]
pub struct ChunkBundle {
    local: ChunkLocal,
    #[bundle]
    mesh_bundle: MeshBundle,
    #[bundle]
    building: ChunkBuildingBundle,
}

impl Default for ChunkBundle {
    fn default() -> Self {
        Self {
            local: ChunkLocal(IVec3::ZERO),
            mesh_bundle: MeshBundle::default(),
            building: ChunkBuildingBundle::default(),
        }
    }
}

#[derive(Bundle)]
pub struct ChunkBuildingBundle {
    faces_occlusion: ChunkFacesOcclusion,
    faces: ChunkFaces,
    vertices: ChunkVertices,
}

impl Default for ChunkBuildingBundle {
    fn default() -> Self {
        Self {
            faces_occlusion: ChunkFacesOcclusion::default(),
            faces: ChunkFaces(vec![]),
            vertices: ChunkVertices(vec![]),
        }
    }
}
