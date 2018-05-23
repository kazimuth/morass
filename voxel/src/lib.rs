//! A simple voxel system, tightly bound to Amethyst.
//!
//! Voxels are stored in chunks, 16x16x16 arrays of voxel information. This is a good data-structure for minecraft-like
//! worlds with large voxels; for worlds with finer voxels you probably want something with better compression.
//!
//! The voxel mesher currently only knows how to draw cubes. Eventually it'll be extended to allow arbitrary meshes for
//! some voxels.
//!
//! If you have something that behaves sort of like a voxel but has a lot of internal state, that should probably be an
//! entity instead.
//!
//! Coordinate system notes:
//!
//! - Individual voxels are always of size 1 in world coordinates.
//!
//! - Their centers are at integer multiples of [1,1,1]; their corners are at those centers offset by .5.
//!
//! - Chunks are meshed such that their world coordinate corresponds to the CENTER of their [0,0,0]th voxel.
//!
//! - World coordinates should be multiples of CHUNK_SIZE_WORLD; i.e. the "starting" chunk is at location 0,0,0,
//!   and the next chunk in the x direction is at CHUNK_SIZE_WORLD,0,0, and so on.

extern crate amethyst;
extern crate cgmath;
#[macro_use]
extern crate log;
extern crate fnv;
extern crate hibitset;
extern crate parking_lot;
extern crate soft_time_limit;
extern crate specs;

use std::fmt::Debug;
use std::ops::{Index, IndexMut};

use amethyst::renderer::{Color, Separate};
use specs::HashMapStorage;
use specs::prelude::*;

pub mod delta;
pub mod mesh;
pub mod raycast;
pub mod tracker;

pub use tracker::ChunkTracker;

// TODO: chunk insertion
// need to mark adjacent chunks for re-meshing, as well

/// A world coordinate as used by Amethyst.
pub type Coord = cgmath::Vector3<f32>;

/// An (integer-vector) coordinate of a voxel.
pub type VoxelCoord = cgmath::Vector3<i16>;

/// Round to the canonical coordinate of the containing voxel, i.e. the center
#[inline(always)]
pub fn canonicalize(coord: Coord) -> VoxelCoord {
    VoxelCoord {
        x: coord.x.round() as i16,
        y: coord.y.round() as i16,
        z: coord.z.round() as i16,
    }
}

/// Round to the canonical coordinate of the containing chunk, i.e. the center of the chunks [0,0,0] voxel
#[inline(always)]
pub fn canonicalize_chunk(coord: VoxelCoord) -> VoxelCoord {
    let coord = VoxelCoord {
        x: coord.x as i16,
        y: coord.y as i16,
        z: coord.z as i16,
    };
    coord - (coord % (CHUNK_SIZE as i16))
}

/// Chunks are CHUNK_SIZE by CHUNK_SIZE by CHUNK_SIZE voxels.
pub const CHUNK_SIZE: usize = 16;
pub const CHUNK_SIZE_WORLD: f32 = CHUNK_SIZE as f32;

/// An individual voxel; will be stored in arrays in chunks.
/// Must be copy: if you want to have stuff in your individual voxels that need heap-allocated stuff,
/// they should be their own entities.
/// Try and keep your voxels as small as possible to reduce memory usage; ideally they'd be 1 byte in size.
pub trait Voxel: Copy + Debug + Default + Send + Sync + 'static {
    fn empty() -> Self;
    fn is_transparent(&self) -> bool;
    /// TODO switch to textures
    fn color(&self) -> Separate<Color>;
}

/// A "voxel chunk" component.
pub struct Chunk<V: Voxel> {
    /// Redundant with transform; both must be set correctly.
    pub coord: VoxelCoord,
    pub voxels: [[[V; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE],
}
impl<V: Voxel> Chunk<V> {
    pub fn empty(coord: VoxelCoord) -> Self {
        let voxel = V::empty();
        let voxels = [[[voxel; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE];
        Chunk { coord, voxels }
    }
    #[inline(always)]
    pub unsafe fn index_unchecked(&self, index: VoxelCoord) -> &V {
        &self.voxels
            .get_unchecked(index.x as usize)
            .get_unchecked(index.y as usize)
            .get_unchecked(index.z as usize)
    }
}
impl<V: Voxel> Index<VoxelCoord> for Chunk<V> {
    type Output = V;

    #[inline(always)]
    fn index(&self, index: VoxelCoord) -> &V {
        &self.voxels[index.x as usize][index.y as usize][index.z as usize]
    }
}
impl<V: Voxel> IndexMut<VoxelCoord> for Chunk<V> {
    #[inline(always)]
    fn index_mut(&mut self, index: VoxelCoord) -> &mut V {
        &mut self.voxels[index.x as usize][index.y as usize][index.z as usize]
    }
}

impl<V: Voxel> Component for Chunk<V> {
    type Storage = FlaggedStorage<Self, HashMapStorage<Self>>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TestVoxel {
    Air,
    Rock,
    Grass,
}
impl Default for TestVoxel {
    fn default() -> Self {
        TestVoxel::Air
    }
}
impl Voxel for TestVoxel {
    fn empty() -> Self {
        TestVoxel::Air
    }
    fn is_transparent(&self) -> bool {
        *self == TestVoxel::Air
    }
    fn color(&self) -> Separate<Color> {
        match *self {
            TestVoxel::Air => Separate::new([0., 0., 0., 0.]),
            TestVoxel::Rock => Separate::new([0.2, 0.2, 0.2, 1.]),
            TestVoxel::Grass => Separate::new([0., 8., 0., 1.]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes() {
        assert!(CHUNK_SIZE < 256);
    }
}
