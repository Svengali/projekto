use bevy::prelude::*;

use crate::world::math;

use super::voxel;

pub const AXIS_SIZE: usize = 16;
// const CHUNK_AXIS_OFFSET: usize = CHUNK_AXIS_SIZE / 2;
pub const BUFFER_SIZE: usize = AXIS_SIZE * AXIS_SIZE * AXIS_SIZE;

pub const X_MASK: usize = 0b_1111_0000_0000;
pub const Z_MASK: usize = 0b_0000_1111_0000;
pub const Y_MASK: usize = 0b_0000_0000_1111;

pub const X_SHIFT: usize = 8;
pub const Z_SHIFT: usize = 4;
pub const Y_SHIFT: usize = 0;

pub struct Chunk {
    voxel_kind: [voxel::Kind; BUFFER_SIZE],
}

impl Default for Chunk {
    fn default() -> Self {
        Self {
            voxel_kind: [0; BUFFER_SIZE],
        }
    }
}

impl Chunk {
    pub fn set_voxel_kind(&mut self, local: IVec3, kind: voxel::Kind) {
        self.voxel_kind[to_index(local)] = kind;
    }

    pub fn get_voxel_kind(&self, local: IVec3) -> voxel::Kind {
        self.voxel_kind[to_index(local)]
    }
}

pub fn to_xyz(index: usize) -> IVec3 {
    IVec3::new(
        ((index & X_MASK) >> X_SHIFT) as i32,
        ((index & Y_MASK) >> Y_SHIFT) as i32,
        ((index & Z_MASK) >> Z_SHIFT) as i32,
    )
}

pub fn to_index(local: IVec3) -> usize {
    let (x, y, z) = local.into();
    (x << X_SHIFT | y << Y_SHIFT | z << Z_SHIFT) as usize
}

pub fn is_within_bounds(pos: IVec3) -> bool {
    math::is_within_cubic_bounds(pos, 0, AXIS_SIZE as i32 - 1)
}

pub fn to_world(local: IVec3) -> Vec3 {
    local.as_f32() * AXIS_SIZE as f32
}

pub fn to_local(world: Vec3) -> IVec3 {
    IVec3::new(
        (world.x / AXIS_SIZE as f32).floor() as i32,
        (world.y / AXIS_SIZE as f32).floor() as i32,
        (world.z / AXIS_SIZE as f32).floor() as i32,
    )
}

#[cfg(test)]
mod tests {
    use bevy::math::IVec3;
    use rand::random;

    #[test]
    fn to_xyz() {
        assert_eq!(IVec3::new(0, 0, 0), super::to_xyz(0));
        assert_eq!(IVec3::new(0, 1, 0), super::to_xyz(1));
        assert_eq!(IVec3::new(0, 2, 0), super::to_xyz(2));

        assert_eq!(IVec3::new(0, 0, 1), super::to_xyz(super::AXIS_SIZE));
        assert_eq!(IVec3::new(0, 1, 1), super::to_xyz(super::AXIS_SIZE + 1));
        assert_eq!(IVec3::new(0, 2, 1), super::to_xyz(super::AXIS_SIZE + 2));

        assert_eq!(
            IVec3::new(1, 0, 0),
            super::to_xyz(super::AXIS_SIZE * super::AXIS_SIZE)
        );
        assert_eq!(
            IVec3::new(1, 1, 0),
            super::to_xyz(super::AXIS_SIZE * super::AXIS_SIZE + 1)
        );
        assert_eq!(
            IVec3::new(1, 2, 0),
            super::to_xyz(super::AXIS_SIZE * super::AXIS_SIZE + 2)
        );

        assert_eq!(
            IVec3::new(1, 0, 1),
            super::to_xyz(super::AXIS_SIZE * super::AXIS_SIZE + super::AXIS_SIZE)
        );
        assert_eq!(
            IVec3::new(1, 1, 1),
            super::to_xyz(super::AXIS_SIZE * super::AXIS_SIZE + super::AXIS_SIZE + 1)
        );
        assert_eq!(
            IVec3::new(1, 2, 1),
            super::to_xyz(super::AXIS_SIZE * super::AXIS_SIZE + super::AXIS_SIZE + 2)
        );
    }

    #[test]
    fn to_index() {
        assert_eq!(super::to_index((0, 0, 0).into()), 0);
        assert_eq!(super::to_index((0, 1, 0).into()), 1);
        assert_eq!(super::to_index((0, 2, 0).into()), 2);

        assert_eq!(super::to_index((0, 0, 1).into()), super::AXIS_SIZE);
        assert_eq!(super::to_index((0, 1, 1).into()), super::AXIS_SIZE + 1);
        assert_eq!(super::to_index((0, 2, 1).into()), super::AXIS_SIZE + 2);

        assert_eq!(
            super::to_index((1, 0, 0).into()),
            super::AXIS_SIZE * super::AXIS_SIZE
        );
        assert_eq!(
            super::to_index((1, 1, 0).into()),
            super::AXIS_SIZE * super::AXIS_SIZE + 1
        );
        assert_eq!(
            super::to_index((1, 2, 0).into()),
            super::AXIS_SIZE * super::AXIS_SIZE + 2
        );

        assert_eq!(
            super::to_index((1, 0, 1).into()),
            super::AXIS_SIZE * super::AXIS_SIZE + super::AXIS_SIZE
        );
        assert_eq!(
            super::to_index((1, 1, 1).into()),
            super::AXIS_SIZE * super::AXIS_SIZE + super::AXIS_SIZE + 1
        );
        assert_eq!(
            super::to_index((1, 2, 1).into()),
            super::AXIS_SIZE * super::AXIS_SIZE + super::AXIS_SIZE + 2
        );
    }

    #[test]
    fn to_world() {
        use super::*;

        const TEST_COUNT: usize = 1000;
        const MAG: f32 = 100.0;

        for _ in 0..TEST_COUNT {
            let base = IVec3::new(
                (random::<f32>() * MAG) as i32 * if random::<bool>() { -1 } else { 1 },
                (random::<f32>() * MAG) as i32 * if random::<bool>() { -1 } else { 1 },
                (random::<f32>() * MAG) as i32 * if random::<bool>() { -1 } else { 1 },
            );

            // To world just convert from local chunk coordinates (1, 2, -1) to world coordinates (16, 32, -16)
            // assuming AXIS_SIZE = 16
            assert_eq!(base.as_f32() * AXIS_SIZE as f32, super::to_world(base));
        }
    }

    #[test]
    fn to_local() {
        use super::*;

        assert_eq!(
            IVec3::new(0, -1, -2),
            super::to_local(Vec3::new(3.0, -0.8, -17.0))
        );
        assert_eq!(
            IVec3::new(0, -1, 0),
            super::to_local(Vec3::new(3.0, -15.8, 0.0))
        );
        assert_eq!(
            IVec3::new(-3, 1, 5),
            super::to_local(Vec3::new(-32.1, 20.0, 88.1))
        );

        const TEST_COUNT: usize = 1000;
        const MAG: f32 = 100.0;

        for _ in 0..TEST_COUNT {
            let base = IVec3::new(
                (random::<f32>() * MAG) as i32 * if random::<bool>() { -1 } else { 1 },
                (random::<f32>() * MAG) as i32 * if random::<bool>() { -1 } else { 1 },
                (random::<f32>() * MAG) as i32 * if random::<bool>() { -1 } else { 1 },
            );

            // This fragment is just used to check if rounding will be correct, since it should not affect
            // the overall chunk local position
            let frag = Vec3::new(
                random::<f32>() * (AXIS_SIZE - 1) as f32,
                random::<f32>() * (AXIS_SIZE - 1) as f32,
                random::<f32>() * (AXIS_SIZE - 1) as f32,
            );

            let world = Vec3::new(
                (base.x * AXIS_SIZE as i32) as f32 + frag.x,
                (base.y * AXIS_SIZE as i32) as f32 + frag.y,
                (base.z * AXIS_SIZE as i32) as f32 + frag.z,
            );

            // To local convert from world chunk coordinates (15.4, 1.1, -0.5) to local coordinates (1, 0, -1)
            // assuming AXIS_SIZE = 16
            assert_eq!(base, super::to_local(world));
        }
    }
}