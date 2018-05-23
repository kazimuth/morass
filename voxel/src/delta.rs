//! A system to apply changes to voxel chunks without blocking everything that requires chunk lookup.
use super::{Voxel, VoxelCoord};

use specs::prelude::*;
use smallvec::SmallVec;

/// A list of pending changes about to be applied to a chunk.
/// Later changes override earlier ones.
struct ChunkDelta<V: Voxel> {
    changes: SmallVec<[(VoxelCoord, V); 16]>
}
impl<V: Voxel> ChunkDelta<V> {
    fn new() -> ChunkDelta<V> {
        ChunkDelta {
            changes: SmallVec::new()
        }
    }
}
impl<V: Voxel> Component for ChunkDelta<V> {
    type Storage = HashMapStorage<Self>;
}

struct ApplyChunkDelta {

}