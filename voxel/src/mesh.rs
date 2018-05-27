//! Creates Amethyst Meshes from voxels.
//!
//! Includes a system to automatically track and re-mesh modified voxels.
//!
//! Meshing takes approx. .15 ms (.00015 s) for a single voxel.

use super::{Chunk, ChunkTracker, Coord, Voxel, VoxelCoord, CHUNK_SIZE};

use std::iter::repeat;
use std::marker::PhantomData;
use std::time::Duration;

use amethyst::assets::{AssetStorage, Handle, Loader};
use amethyst::renderer::{Color, ComboMeshCreator, Material, Mesh, Normal, Position, Separate, MaterialDefaults};
use cgmath::Vector3;
use hibitset::BitSetLike;
use soft_time_limit::TimeLimiter;
use specs::prelude::*;

pub struct InProgress {
    pub color: Vec<Separate<Color>>,
    pub position: Vec<Separate<Position>>,
    pub normal: Vec<Separate<Normal>>,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Direction {
    East = 0,
    Up = 1,
    North = 2,
    West = 3,
    Down = 4,
    South = 5,
}
impl Direction {
    pub fn all() -> [Direction; 6] {
        use self::Direction::*;
        [East, Up, North, West, Down, South]
    }
}

// directions for meshing
// we've chosen things carefully so that the order
// [(iter1, iter2), (-iter1, iter2), (-iter1, -iter2), (iter1, -iter2)]
// is a CCW winding with normal NORMAL;
// and all iteration through arrays is either both-forward or both-backward.
// the array iteration order matters 'cause we'll do greedy meshing later;
// and I want the points to make sense when being greedy.
const NORMALS: [VoxelCoord; 6] = [
    VoxelCoord { x: 1, y: 0, z: 0 },
    VoxelCoord { x: 0, y: 1, z: 0 },
    VoxelCoord { x: 0, y: 0, z: 1 },
    VoxelCoord { x: -1, y: 0, z: 0 },
    VoxelCoord { x: 0, y: -1, z: 0 },
    VoxelCoord { x: 0, y: 0, z: -1 },
];
const ITERS: [(VoxelCoord, VoxelCoord); 6] = [
    (
        VoxelCoord { x: 0, y: 1, z: 0 },
        VoxelCoord { x: 0, y: 0, z: 1 },
    ),
    (
        VoxelCoord { x: 1, y: 0, z: 0 },
        VoxelCoord { x: 0, y: 0, z: 1 },
    ),
    (
        VoxelCoord { x: 1, y: 0, z: 0 },
        VoxelCoord { x: 0, y: 1, z: 0 },
    ),
    (
        VoxelCoord { x: 0, y: -1, z: 0 },
        VoxelCoord { x: 0, y: 0, z: -1 },
    ),
    (
        VoxelCoord { x: -1, y: 0, z: 0 },
        VoxelCoord { x: 0, y: 0, z: -1 },
    ),
    (
        VoxelCoord { x: -1, y: 0, z: 0 },
        VoxelCoord { x: 0, y: -1, z: 0 },
    ),
];
const BACKWARDS: [bool; 6] = [false, false, false, true, true, true];

/// Mesh a single direction of a single layer.
///
/// direction is in (1 - 6)
///
/// TODO: greedy meshing for this layer
pub fn mesh_layer<V: Voxel>(
    chunk1: &Chunk<V>,
    level1: i16,
    chunk2: &Chunk<V>,
    level2: i16,
    direction: Direction,
    in_progress: &mut InProgress,
) {
    let direction = direction as usize;
    let normal = NORMALS[direction];
    let axis = VoxelCoord {
        x: normal.x.abs(),
        y: normal.y.abs(),
        z: normal.z.abs(),
    };
    let (iter1, iter2) = ITERS[direction];
    let backwards = BACKWARDS[direction];

    let halfnormalf: Vector3<f32> = normal.cast().unwrap() * 0.5;

    let offset1 = axis * level1;
    let offset2 = axis * level2;

    let iter1f: Coord = iter1.cast().unwrap();
    let iter2f: Coord = iter2.cast().unwrap();
    let positions = [
        (iter1f + iter2f),
        (-iter1f + iter2f),
        (-iter1f - iter2f),
        (iter1f - iter2f),
        (iter1f + iter2f),
        (-iter1f - iter2f),
    ];
    let normal_f: Separate<Normal> =
        Separate::new([normal.x as f32, normal.y as f32, normal.z as f32]);

    let initlen = in_progress.color.len();

    // This loop currently takes around 10ns per voxel, it's not likely to be a bottleneck
    let mut row = if backwards {
        -(CHUNK_SIZE as i16 - 1) * iter1
    } else {
        VoxelCoord::new(0, 0, 0)
    };

    for _ in 0..CHUNK_SIZE {
        let mut loc = row + if backwards {
            -(CHUNK_SIZE as i16 - 1) * iter2
        } else {
            VoxelCoord::new(0, 0, 0)
        };
        for _ in 0..CHUNK_SIZE {
            let loc1 = offset1 + loc;
            let loc2 = offset2 + loc;
            let kind1 = unsafe { chunk1.index_unchecked(loc1) };
            let kind2 = unsafe { chunk2.index_unchecked(loc2) };

            if !kind1.is_transparent() && kind2.is_transparent() {
                // we have a boundary
                let face_center: Vector3<f32> = loc1.cast().unwrap() + halfnormalf;

                for p in positions.iter() {
                    in_progress.color.push(Separate::new(kind1.color()));
                    in_progress
                        .position
                        .push(Separate::new((face_center + p).into()));
                }
            }
            loc += iter2;
        }
        row += iter1;
    }
    let n = in_progress.position.len() - initlen;

    in_progress.normal.extend(repeat(normal_f).take(n));
}

pub fn mesh_chunk<V: Voxel>(
    coord: VoxelCoord,
    tracker: &ChunkTracker,
    chunks: &ReadStorage<Chunk<V>>,
) -> ComboMeshCreator {
    let mut result = InProgress {
        color: Vec::new(),
        position: Vec::new(),
        normal: Vec::new(),
    };
    let center = tracker
        .get_chunk(chunks, coord)
        .expect("can't mesh nonexistent chunk!");

    let empty = Chunk {
        coord: VoxelCoord::new(0, 0, 0),
        voxels: [[[V::default(); CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE],
    };

    for direction in Direction::all().into_iter() {
        let i = *direction as usize;

        let (start, end, sub) = if BACKWARDS[i] {
            (1, CHUNK_SIZE as i16, -1)
        } else {
            (0, CHUNK_SIZE as i16 - 1, 1)
        };

        // mesh interior faces
        for offset in start..end {
            mesh_layer(
                center,
                offset,
                center,
                offset + sub,
                *direction,
                &mut result,
            );
        }
        let adjacent_coord = coord + NORMALS[i] * CHUNK_SIZE as i16;
        let adjacent = tracker.get_chunk(chunks, adjacent_coord).unwrap_or(&empty);

        let (center_layer, adjacent_layer) = if BACKWARDS[i] {
            (0, CHUNK_SIZE as i16 - 1)
        } else {
            (CHUNK_SIZE as i16 - 1, 0)
        };
        mesh_layer(center, center_layer, adjacent, adjacent_layer, *direction, &mut result);
    }

    let InProgress {
        position,
        color,
        normal,
    } = result;

    (position, Some(color), None, Some(normal), None).into()
}

/// Tracks modified voxels and re-meshes them.
///
/// Note that this uses specs' FlaggedStorage, which means that
/// whenever you take a &mut chunk, that chunk is marked as modified.
/// This means that you should never mutably iterate all chunks!
/// Only mutably take a chunk if you're actually modifying it.
/// Otherwise you'll just re-mesh everything.
pub struct ChunkMesherSystem<V: Voxel> {
    time_limiter: TimeLimiter,
    time_limit: Duration,
    ids: Option<(ReaderId<InsertedFlag>, ReaderId<ModifiedFlag>, ReaderId<RemovedFlag>)>,
    to_do: BitSet,
    _phantom: PhantomData<V>,
}

impl<V: Voxel> ChunkMesherSystem<V> {
    pub fn new(time_limit: Duration) -> Self {
        ChunkMesherSystem {
            ids: None,
            time_limiter: TimeLimiter::new(),
            time_limit,
            to_do: BitSet::new(),
            _phantom: PhantomData,
        }
    }
}

impl<'a, V: Voxel> System<'a> for ChunkMesherSystem<V> {
    type SystemData = (
        Entities<'a>,
        ReadExpect<'a, ChunkTracker>,
        ReadExpect<'a, Loader>,
        ReadExpect<'a, AssetStorage<Mesh>>,
        ReadExpect<'a, MaterialDefaults>,
        ReadStorage<'a, Chunk<V>>,
        WriteStorage<'a, Handle<Mesh>>,
        WriteStorage<'a, Material>,
    );

    fn setup(&mut self, resources: &mut Resources) {
        Self::SystemData::setup(resources);
        let mut chunks = WriteStorage::<Chunk<V>>::fetch(resources);
        self.ids = Some((chunks.track_inserted(), chunks.track_modified(), chunks.track_removed()));
    }

    fn run(
        &mut self,
        (entities, tracker, loader, assets, mat, chunks, mut meshes, mut materials): Self::SystemData,
    ) {
        let &mut (ref mut inserted_ids, ref mut modified_ids, ref mut removed_ids) = self.ids.as_mut().unwrap();
        chunks.populate_inserted(inserted_ids, &mut self.to_do);
        chunks.populate_modified(modified_ids, &mut self.to_do);
        for removed in chunks.removed().read(removed_ids) {
            let idx = **removed;
            self.to_do.remove(idx);
        }

        let mut completed = Vec::new();
        {
            let mut iter = (&self.to_do).iter();
            self.time_limiter.repeat_with_budget(self.time_limit, || {
                if let Some(idx) = iter.next() {
                    let ent = entities.entity(idx);
                    info!("meshing {:?}", ent);
                    let chunk = chunks.get(ent);
                    if let None = chunk {
                        return true;
                    }
                    let chunk = chunk.unwrap();
                    let pre_mesh = mesh_chunk(chunk.coord, &*tracker, &chunks);
                    let mesh: Handle<Mesh> = loader.load_from_data(pre_mesh.into(), (), &*assets);

                    let _ = meshes
                        .insert(ent, mesh)
                        .map_err(|e| error!("mesh insertion failed! {:?}", e));
                    let _ = materials
                        .insert(ent, mat.0.clone())
                        .map_err(|_| error!("material insertion failed!"));

                    completed.push(idx);

                    info!("meshed {:?}", ent);
                    true
                } else {
                    false
                }
            });
        }

        for done in completed {
            self.to_do.remove(done);
        }
    }
}
