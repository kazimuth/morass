//! Implements a system to allow lookups of chunks by coordinate.

use super::{canonicalize_chunk, Chunk, Voxel, VoxelCoord};

use fnv::FnvHashMap;
use specs::prelude::*;
use specs::world::Index;
use std::marker::PhantomData;

/// A global table of chunks, to allow easy lookup of neighbors.
/// Doesn't track chunk movement; if you reassign a chunk location nothing will happen.
#[derive(Default, Debug)]
pub struct ChunkTracker {
    // bidirectional mapping
    coord_to_ent: FnvHashMap<VoxelCoord, Entity>,
    idx_to_coord: FnvHashMap<Index, VoxelCoord>,
}
impl ChunkTracker {
    pub fn new() -> Self {
        Default::default()
    }

    #[inline(always)]
    pub fn get_chunk_ent(&self, coord: VoxelCoord) -> Option<Entity> {
        self.coord_to_ent
            .get(&canonicalize_chunk(coord))
            .map(Clone::clone)
    }

    pub fn get_chunk<'a, V: Voxel>(
        &self,
        chunks: &'a ReadStorage<Chunk<V>>,
        coord: VoxelCoord,
    ) -> Option<&'a Chunk<V>> {
        self.get_chunk_ent(coord).and_then(|ent| chunks.get(ent))
    }
}

/// A system that registers new chunks in the ChunkTracker.
pub struct ChunkTrackerSystem<V: Voxel> {
    inserted_ids: ReaderId<InsertedFlag>,
    removed_ids: ReaderId<RemovedFlag>,
    _phantom: PhantomData<V>,
}
impl<V: Voxel> ChunkTrackerSystem<V> {
    pub fn for_world(world: &World) -> Self {
        let mut chunks = world.write_storage::<Chunk<V>>();
        let inserted_ids = chunks.track_inserted();
        let removed_ids = chunks.track_removed();

        ChunkTrackerSystem {
            inserted_ids,
            removed_ids,
            _phantom: PhantomData,
        }
    }
}
impl<'a, V: Voxel> System<'a> for ChunkTrackerSystem<V> {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, Chunk<V>>,
        Write<'a, ChunkTracker>,
    );

    fn run(&mut self, (entities, chunks, mut tracker): Self::SystemData) {
        for removed in chunks.removed().read(&mut self.removed_ids) {
            let idx = **removed;
            let coord = *tracker
                .idx_to_coord
                .get(&idx)
                .expect("removed but not present");

            debug_assert!(tracker.idx_to_coord.contains_key(&idx));
            debug_assert!(tracker.coord_to_ent.contains_key(&coord));

            tracker.idx_to_coord.remove(&idx);
            tracker.coord_to_ent.remove(&coord);
        }
        for inserted in chunks.inserted().read(&mut self.inserted_ids) {
            let idx = **inserted;
            let ent = entities.entity(idx);
            let coord = chunks.get(ent).expect("inserted but not present").coord;

            debug_assert!(!tracker.idx_to_coord.contains_key(&idx));
            debug_assert!(!tracker.coord_to_ent.contains_key(&coord));

            tracker.idx_to_coord.insert(idx, coord);
            tracker.coord_to_ent.insert(coord, ent);
        }
    }
}

/*
use super::{Voxel, VoxelCoord, Chunk, ChunkTracker};

use std::marker::PhantomData;

use specs::{Read, ReadStorage, WriteStorage, Entity};

const LRU_SIZE: usize = 4;

/// An LRU cache for voxels. Makes repeated voxel lookups cheaper, assuming you're
/// doing nearby lookups:
/// 
/// Before: conversion + hash(coord) + deref + hash(ent) + deref + offset
/// 
/// After: conversion + comparison + deref + offset
/// 
/// If you're doing a LOT of reads from the same chunk, you should probably still
/// lookup the chunks and do the offsets yourself (see e.g. meshing.)
pub struct Lookup<'a: 'b, 'b, V: Voxel> {
    storage: *const ReadStorage<'a, Chunk<V>>,
    _borrow: PhantomData<&'b ReadStorage<'a, Chunk<V>>>,
    tracker: &'b Read<'a, ChunkTracker>,
    slots: [*const Chunk<V>; LRU_SIZE],
    coords: [VoxelCoord; LRU_SIZE],
    // playing it safe WRT: UB
}
impl<'a: 'b, 'b, V: Voxel> Lookup<'a, 'b, V> {
    
}
*/

#[cfg(test)]
mod tests {
    use super::*;
    use TestVoxel;

    #[test]
    fn test_tracker() {
        let mut world = World::new();
        world.register::<Chunk<TestVoxel>>();
        world.add_resource(ChunkTracker::new());

        let mut dispatcher = DispatcherBuilder::new()
            .with(
                ChunkTrackerSystem::<TestVoxel>::for_world(&world),
                "chunk_tracker",
                &[],
            )
            .build();

        dispatcher.dispatch(&mut world.res);

        // add entity
        let coord = VoxelCoord::new(0, 0, 0);
        let ent = world
            .create_entity()
            .with(Chunk::<TestVoxel>::empty(coord))
            .build();
        dispatcher.dispatch(&mut world.res);
        {
            let tracker = world.read_resource::<ChunkTracker>();
            assert_eq!(tracker.get_chunk_ent(VoxelCoord::new(0, 0, 0)), Some(ent));
        }

        // remove entity
        world.delete_entity(ent).unwrap();
        dispatcher.dispatch(&mut world.res);
        {
            let tracker = world.read_resource::<ChunkTracker>();
            assert_eq!(tracker.get_chunk_ent(VoxelCoord::new(0, 0, 0)), None);
        }
    }
}
