//! A system to apply changes to voxel chunks without blocking everything that requires chunk lookup.
use super::{canonicalize_chunk, Chunk, ChunkTracker, Voxel, VoxelCoord};

use parking_lot::Mutex;
use specs::prelude::*;
use std::marker::PhantomData;

#[derive(Default)]
pub struct ChunkDeltas<V: Voxel> {
    pending: Mutex<Vec<(VoxelCoord, V)>>,
}
impl<V: Voxel> ChunkDeltas<V> {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn defer_set(&self, coord: VoxelCoord, voxel: V) {
        self.pending.lock().push((coord, voxel));
    }
}
#[derive(Default)]
pub struct ChunkDeltaSystem<V: Voxel> {
    _phantom: PhantomData<V>,
}
impl<V: Voxel> ChunkDeltaSystem<V> {
    pub fn new() -> Self {
        Default::default()
    }
}
impl<'a, V: Voxel> System<'a> for ChunkDeltaSystem<V> {
    type SystemData = (
        Read<'a, ChunkTracker>,
        // not that we actually need write access, but:
        // this locks the deltas and ensures that they're applied at a consistent time
        // each frame.
        Write<'a, ChunkDeltas<V>>,
        WriteStorage<'a, Chunk<V>>,
    );

    fn run(&mut self, (tracker, deltas, mut chunks): Self::SystemData) {
        let mut pending = deltas.pending.lock();
        for (coord, voxel) in pending.drain(0..) {
            let canon = canonicalize_chunk(coord);
            // TODO error handling
            let ent = tracker.get_chunk_ent(canon);
            if let Some(ent) = ent {
                let chunk = chunks.get_mut(ent).unwrap();
                chunk[coord - canon] = voxel;
            } else {
                error!(
                    "no chunk entity found for defer_set coord: {:?} voxel: {:?}, ignoring",
                    coord, voxel
                );
                continue;
            }
        }
    }
}
