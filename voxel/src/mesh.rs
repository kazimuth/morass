use super::{Chunk, Coord, ChunkCoord, Voxel, TestVoxel, CHUNK_SIZE};
use amethyst::renderer::{Separate, Color, Position, Normal, Tangent};
use cgmath::Vector3;
use std::iter::repeat;

pub struct InProgress {
    pub color: Vec<Separate<Color>>,
    pub position: Vec<Separate<Position>>,
    pub normal: Vec<Separate<Normal>>,
    pub tangent: Vec<Separate<Tangent>>
}

/// Mesh a single direction of a single layer.
/// 
/// axis is a unit basis vector
/// normal points from level1 to level2
/// 
/// this is probably wrong
/// 
/// TODO: greedy meshing for this layer
pub fn mesh_layer<V: Voxel>(chunk1: &Chunk<V>, level1: i16,
                        chunk2: &Chunk<V>, level2: i16,
                        axis: ChunkCoord,
                        normal: ChunkCoord,
                        in_progress: &mut InProgress) {

    let (tan1, tan2) = if axis.x == 1 {
        (ChunkCoord::unit_y(), ChunkCoord::unit_z())
    } else if axis.y == 1 {
        (ChunkCoord::unit_x(), ChunkCoord::unit_z())
    } else {
        (ChunkCoord::unit_y(), ChunkCoord::unit_x())
    };

    let halfnormalf: Vector3<f32> = normal.cast().unwrap() * 0.5;

    let offset1 = axis * level1;
    let offset2 = axis * level2;

    let tan1f: Coord = tan1.cast().unwrap();
    let tan2f: Coord = tan2.cast().unwrap();
    let positions = [
        // TODO CW or CCW?
        -0.5 * tan1f - 0.5 * tan2f,
        -0.5 * tan1f + 0.5 * tan2f,
         0.5 * tan1f - 0.5 * tan2f,

         0.5 * tan1f + 0.5 * tan2f,
         0.5 * tan1f - 0.5 * tan2f,
        -0.5 * tan1f + 0.5 * tan2f,
    ];
    let normal_f: Separate<Normal> = Separate::new([normal.x as f32, normal.y as f32, normal.z as f32]);
    let tan_f: Separate<Tangent> = Separate::new([tan1f.x, tan1f.y, tan1f.z]);

    let mut initlen = in_progress.color.len();

    let mut row = ChunkCoord::new(0, 0, 0);
    for _ in 0..CHUNK_SIZE {
        let mut loc = row;
        for _ in 0..CHUNK_SIZE {
            let loc1 = offset1 + loc;
            let loc2 = offset2 + loc;
            let kind1 = unsafe { chunk1.index_unchecked(loc1) };
            let kind2 = unsafe { chunk2.index_unchecked(loc2) };

            if !kind1.is_empty() && kind2.is_empty() {
                // we have a boundary
                let face_center: Vector3<f32> = loc1.cast().unwrap() + halfnormalf;

                for p in positions.iter() {
                    in_progress.color.push(kind1.color());
                    in_progress.position.push(Separate::new((face_center + p).into()));
                }
            }
            loc += tan2;
        }
        row += tan1;
    }
    let n = in_progress.position.len() - initlen;

    in_progress.normal.extend(repeat(normal_f).take(n));
    in_progress.tangent.extend(repeat(tan_f).take(n));
}