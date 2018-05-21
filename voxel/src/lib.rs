//! A simple voxel system, tightly bound to Amethyst.
//! 
//! Voxels are stored in chunks, large blocks of 
//! 
//! Coordinate system notes:
//! Individual voxels are always of size 1 in world coordinates.
//! Their centers are at integer multiples of [1,1,1]; their corners are at those centers offset by .5.
//! Chunks are meshed such that their world coordinate corresponds to the CENTER of their [0,0,0]th voxel.
//! World coordinates should be multiples of CHUNK_SIZE_WORLD; i.e. the "starting" chun is at location 0,0,0,
//! and the next chunk in the x direction is at CHUNK_SIZE_WORLD,0,0, and so on.

extern crate cgmath;
extern crate fnv;
extern crate specs;
extern crate amethyst;

use std::marker::PhantomData;

use specs::prelude::*;
use specs::HashMapStorage;
use specs::world::Index;
use fnv::FnvHashMap;

use amethyst::renderer::{Separate, Color};

pub mod mesh;

/// A world coordinate.
pub type Coord = cgmath::Vector3<f32>;

/// A coordinate of a chunk.
/// Note that these are multiples of CHUNK_SIZE_WORLD.
pub type ChunkCoord = cgmath::Vector3<i16>;

/// Round to the canonical coordinate of the containing voxel, i.e. the center
#[inline(always)]
pub fn canonicalize(coord: Coord) -> Coord {
    Coord { x: coord.x.round(), y: coord.y.round(), z: coord.z.round() }
}

/// Round to the canonical coordinate of the containing chunk, i.e. the center of the chunks [0,0,0] voxel
#[inline(always)]
pub fn canonicalize_chunk(coord: Coord) -> ChunkCoord {
    let coord = canonicalize(coord);
    let coord = ChunkCoord { x: coord.x as i16, y: coord.y as i16, z: coord.z as i16 };
    coord - (coord % (CHUNK_SIZE as i16))
}

/// Chunks are CHUNK_SIZE by CHUNK_SIZE by CHUNK_SIZE voxels.
pub const CHUNK_SIZE: usize = 16;
pub const CHUNK_SIZE_WORLD: f32 = CHUNK_SIZE as f32;

/// An individual voxel; will be stored in arrays in chunks.
/// Must be copy: if you want to have stuff in your individual voxels that need heap-allocated stuff,
/// they should be their own entities.
/// Try and keep your voxels as small as possible to reduce memory usage; ideally they'd be 1 byte in size.
pub trait Voxel: Copy + Send + Sync + 'static {
    fn empty() -> Self;
    fn is_empty(&self) -> bool;
    /// TODO switch to textures
    fn color(&self) -> Separate<Color>;
}

/// A "voxel chunk" component.
pub struct Chunk<V: Voxel> {
    /// Redundant with transform; both must be set correctly.
    pub coord: ChunkCoord,
    pub voxels: [[[V; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE]
}
impl<V: Voxel> Chunk<V> {
    pub fn empty(coord: ChunkCoord) -> Self {
        let voxel = V::empty();
        let voxels = [[[voxel; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE];
        Chunk { coord, voxels }
    }
    #[inline(always)]
    fn index(&self, index: ChunkCoord) -> &V {
        &self.voxels[index.x as usize][index.y as usize][index.z as usize]
    }
    #[inline(always)]
    unsafe fn index_unchecked(&self, index: ChunkCoord) -> &V {
        &self.voxels.get_unchecked(index.x as usize)
                    .get_unchecked(index.y as usize)
                    .get_unchecked(index.z as usize)
    }

}
impl<V: Voxel> Component for Chunk<V> {
    type Storage = FlaggedStorage<Self, HashMapStorage<Self>>;
}

/// A global table of chunks, to allow easy lookup of neighbors.
/// DOES NOT TRACK CHUNK MOVEMENT. idk why you'd even want that though.
#[derive(Default, Debug)]
pub struct ChunkTrackerResource {
    // bidirectional mapping
    coord_to_idx: FnvHashMap<ChunkCoord, Index>,
    idx_to_coord: FnvHashMap<Index, ChunkCoord>
}
impl ChunkTrackerResource {
    fn new() -> Self {
        Default::default()
    }

    #[inline(always)]
    fn get_chunk_idx_at_coord(&self, coord: Coord) -> Option<Index> {
        self.coord_to_idx.get(&canonicalize_chunk(coord)).map(Clone::clone)
    }
}

/// A system that registers new chunks in the ChunkTrackerResource.
pub struct ChunkTrackerSystem<V: Voxel> {
    inserted_ids: ReaderId<InsertedFlag>,
    removed_ids: ReaderId<RemovedFlag>,
    _phantom: PhantomData<V>
}
impl<V: Voxel> ChunkTrackerSystem<V> {
    fn for_world(world: &World) -> Self {
        let mut chunks = world.write_storage::<Chunk<V>>();
        let mut inserted_ids = chunks.track_inserted();
        let mut removed_ids = chunks.track_removed();

        ChunkTrackerSystem {
            inserted_ids, removed_ids, _phantom: PhantomData
        }
    }
}

impl<'a, V: Voxel> System<'a> for ChunkTrackerSystem<V> {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, Chunk<V>>,
        Write<'a, ChunkTrackerResource>
    );

    fn run(&mut self, (entities, chunks, mut tracker): Self::SystemData) {
        for removed in chunks.removed().read(&mut self.removed_ids) {
            let idx = **removed;
            let coord = *tracker.idx_to_coord.get(&idx).expect("removed but not present");

            debug_assert!(tracker.idx_to_coord.contains_key(&idx));
            debug_assert!(tracker.coord_to_idx.contains_key(&coord));

            tracker.idx_to_coord.remove(&idx);
            tracker.coord_to_idx.remove(&coord);
        }
        for inserted in chunks.inserted().read(&mut self.inserted_ids) {
            let idx = **inserted;
            let coord = chunks.get(entities.entity(idx)).expect("inserted but not present").coord;

            debug_assert!(!tracker.idx_to_coord.contains_key(&idx));
            debug_assert!(!tracker.coord_to_idx.contains_key(&coord));

            tracker.idx_to_coord.insert(idx, coord);
            tracker.coord_to_idx.insert(coord, idx);
        }
    }
}

pub struct ChunkMesherSystem {
    inserted_ids: ReaderId<InsertedFlag>,
    modified_ids: ReaderId<InsertedFlag>,
    scratch: BitSet,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TestVoxel {
    Air,
    Rock,
    Grass
}
impl Voxel for TestVoxel {
    fn empty() -> Self {
        TestVoxel::Air
    }
    fn is_empty(&self) -> bool {
        *self == TestVoxel::Air
    }
    fn color(&self) -> Separate<Color> {
        match *self {
            TestVoxel::Air => Separate::new([0.,0.,0.,0.]),
            TestVoxel::Rock => Separate::new([0.2,0.2,0.2,1.]),
            TestVoxel::Grass => Separate::new([0.,8.,0.,1.])
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

    #[test]
    fn test_tracker() {
        let mut world = World::new();
        world.register::<Chunk<TestVoxel>>();
        world.add_resource(ChunkTrackerResource::new());

        let mut dispatcher = DispatcherBuilder::new()
            .with(ChunkTrackerSystem::<TestVoxel>::for_world(&world), "chunk_tracker", &[])
            .build();

        dispatcher.dispatch(&mut world.res);

        // add entity
        let coord = ChunkCoord::new(0, 0, 0);
        let ent = world.create_entity().with(Chunk::<TestVoxel>::empty(coord)).build();
        dispatcher.dispatch(&mut world.res);
        {
            let tracker = world.read_resource::<ChunkTrackerResource>();
            assert_eq!(tracker.get_chunk_idx_at_coord(Coord::new(0., 0., 0.)), Some(ent.id()));
        }

        // remove entity
        world.delete_entity(ent).unwrap();
        dispatcher.dispatch(&mut world.res);
        {
            let tracker = world.read_resource::<ChunkTrackerResource>();
            assert_eq!(tracker.get_chunk_idx_at_coord(Coord::new(0., 0., 0.)), None);
        }
    }
}
