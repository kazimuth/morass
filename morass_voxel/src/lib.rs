extern crate voxel;

pub use voxel::*;

pub type MorassChunk = Chunk<MorassVoxel>;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MorassVoxel {
    Air,
    Grass,
    Stone,
    Wood
}
impl Default for MorassVoxel {
    fn default() -> Self {
        MorassVoxel::Air
    }
}
impl Voxel for MorassVoxel {
    fn is_transparent(&self) -> bool {
        *self == MorassVoxel::Air
    }
    fn color(&self) -> [f32; 4] {
        match *self {
            MorassVoxel::Air => [0.0, 0.0, 0.0, 0.0],
            MorassVoxel::Grass => [118.0/255.0, 166.0/255.0, 70.0/255.0, 0.0],
            MorassVoxel::Stone => [132.0/255.0,116.0/255.0,119.0/255.0, 0.0],
            MorassVoxel::Wood => [92.0/255.0,44.0/255.0,29.0/255.0, 0.0],
        }
    }
}