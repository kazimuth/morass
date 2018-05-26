//! This module implements the fast voxel traversal algorithm from:
//! "A Fast Voxel Traversal Algorithm for Ray Tracing", John Amanatides, Andrew Woo, 1987
//! http://citeseerx.ist.psu.edu/viewdoc/download?doi=10.1.1.42.3443&rep=rep1&type=pdf

use super::{canonicalize, canonicalize_chunk, Coord, VoxelCoord, Voxel, Chunk, ChunkTracker, CHUNK_SIZE};
use std::f32;
use cgmath::InnerSpace;
use specs::ReadStorage;

/// The face a raycasting operation hit.
/// 
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FaceHit {
    X, Y, Z, Contained
}

/// Information about a cast ray.
#[derive(Clone, Copy, Debug)]
pub struct Raycast {
    /// The face the ray hit.
    face_hit: FaceHit,
    /// The ending point of the ray.
    /// Note: may be slightly outside `end_voxel` due to floating point error.
    end: Coord,
    /// The voxel the ray ended on.
    end_voxel: VoxelCoord,
    /// Whether the voxel we hit was "interesting", i.e.
    /// if this is false, we hit the border of the voxel.
    hit_interesting: bool,
}

/// Starting at "start_voxel" / "start", walk through grid squares
/// until the current voxel goes outside the cube defined by:
///  (min.x, max.x) x (min.y, max.y) x (min.z, max.z)
/// (that is, inclusive)
/// 
/// `start_voxel` and `start` are redundant:
/// this is to allow starting coordinates on the edge of a voxel
/// to work correctly. `start` should be within `start_voxel` for the
/// algorithm to work correctly; although it can be a small amount outside due
/// to e.g. floating point error.
///
/// is_interesting should return "true" to signal that the raycast should stop.
/// it will not be evaluated for border voxels, that is, voxels where x == min.x and so on.
///
/// Returns a multiple of the direction vector that puts it in the target voxel,
/// and the coordinate of the voxel that occluded the ray.
/// 
/// Note that voxels are centered at integer coordinates.
#[inline]
pub fn raycast<F: FnMut(VoxelCoord) -> bool>(
    start_voxel: VoxelCoord,
    start: Coord,
    direction: Coord,
    min: VoxelCoord,
    max: VoxelCoord,
    mut is_interesting: F,
) -> Raycast {
    // if we're in a target block, return immediately
    if is_interesting(start_voxel) {
        return Raycast {
            face_hit: FaceHit::Contained,
            end: start,
            end_voxel: start_voxel,
            hit_interesting: true
        };
    }

    // integer coordinates of the center of our voxel
    let VoxelCoord {
        mut x,
        mut y,
        mut z,
    } = start_voxel;
    let step = canonicalize(direction);
    let (step_x, step_y, step_z) = (step.x.signum(), step.y.signum(), step.z.signum());

    assert!(!start.x.is_nan() && !start.y.is_nan() && !start.z.is_nan());
    assert!(!direction.x.is_nan() && !direction.y.is_nan() && !direction.z.is_nan());
    assert!(min.x <= x && x <= max.x);
    assert!(min.y <= y && x <= max.y);
    assert!(min.z <= z && x <= max.z);

    // box defining stopping voxels
    let lim_x =
        if step_x > 0 { max.x } else { min.x };
    let lim_y =
        if step_y > 0 { max.y } else { min.y };
    let lim_z =
        if step_z > 0 { max.z } else { min.z };

    // floating point coordinates
    // t_max_c: multiple of direction to get to that edge of voxel
    // t_dc: multiple of direction to move 1 voxel
    let Coord {
        x: dx,
        y: dy,
        z: dz,
    } = direction;
    let (mut t_max_x, t_dx) = init(start_voxel.x, start.x, dx);
    let (mut t_max_y, t_dy) = init(start_voxel.y, start.y, dy);
    let (mut t_max_z, t_dz) = init(start_voxel.z, start.z, dz);

    loop {
        if t_max_x <= t_max_y && t_max_x <= t_max_z {
            x += step_x;
            let cur = VoxelCoord { x, y, z };

            let hit_border = x == lim_x;
            // note: evaluation order is important here:
            // we don't want to evaluate the predicate on border voxels
            if hit_border || is_interesting(cur) {
                return Raycast {
                    face_hit: FaceHit::X,
                    end: start + direction * t_max_x,
                    end_voxel: cur,
                    // if we hit the border, it can't be interesting;
                    // if we hit interesting, it can't be border
                    hit_interesting: !hit_border,
                };
            }

            t_max_x += t_dx;
        } else if t_max_y < t_max_x && t_max_y <= t_max_z {
            y += step_y;
            let cur = VoxelCoord { x, y, z };

            let hit_border = y == lim_y;
            if hit_border || is_interesting(cur) {
                return Raycast {
                    face_hit: FaceHit::Y,
                    end: start + direction * t_max_y,
                    end_voxel: cur,
                    hit_interesting: !hit_border,
                };
            }

            t_max_y += t_dy;
        } else {
            z += step_z;
            let cur = VoxelCoord { x, y, z };

            let hit_border = z == lim_z;
            if hit_border || is_interesting(cur) {
                return Raycast {
                    face_hit: FaceHit::Z,
                    end: start + direction * t_max_z,
                    end_voxel: cur,
                    hit_interesting: !hit_border,
                };
            }

            t_max_z += t_dz;
        }
    }
}

fn init(v_c: i16, c: f32, dc: f32) -> (f32, f32) {
    let max_c = v_c as f32 + dc.signum() * 0.5;
    let mut t_max_c = (max_c - c) / dc;
    if t_max_c < 0.0 {
        t_max_c = f32::INFINITY;
    }
    let t_dc = 1.0 / dc;
    (t_max_c, t_dc)
}

const SIZE_F: f32 = CHUNK_SIZE as f32;
const SIZE_I: i16 = CHUNK_SIZE as i16;

/// Raycast through a voxel world looking for a non-empty voxel.
pub fn voxel_raycast<V: Voxel>(
    tracker: &ChunkTracker,
    storage: &ReadStorage<Chunk<V>>,
    coord: Coord,
    direction: Coord,
    min_chunk: VoxelCoord,
    max_chunk: VoxelCoord
) -> Raycast {
    let start_coord_v = coord;

    // algorithm: raycast through chunks until we find a non-empty one;
    // then, raycast through voxels until we find a non-empty one or leave the chunk;
    // repeat.

    // we use a special coordinate system for the chunk raycasting,
    // because `raycast` only works if voxel centers are integers.
    // see below for the transformations made into that coordinate system.

    // the following are in voxel space:
    let mut cur_coord_v = coord;
    let mut cur_voxel_v = canonicalize(coord);

    let min_chunk_v = min_chunk;
    let max_chunk_v = max_chunk;

    // these are in chunk space
    let min_chunk_c = min_chunk / SIZE_I;
    let max_chunk_c = max_chunk / SIZE_I;

    loop {
        // in voxel space, NOT chunk space
        let cur_chunk_v = canonicalize_chunk(cur_voxel_v);

        if let Some(chunk) = tracker.get_chunk(storage, cur_chunk_v) {
            // raycast through voxel space
            let hit = raycast(cur_voxel_v, cur_coord_v, direction,
                // set bounds outside this voxel
                cur_chunk_v - VoxelCoord::new(-1,-1,-1),
                cur_chunk_v + VoxelCoord::new(SIZE_I, SIZE_I, SIZE_I),
                |v| !chunk[v - cur_chunk_v].is_transparent()
            );

            if hit.hit_interesting ||
                hit.end_voxel.x <= min_chunk_v.x ||
                hit.end_voxel.y <= min_chunk_v.y ||
                hit.end_voxel.z <= min_chunk_v.z ||
                hit.end_voxel.x >= max_chunk_v.x ||
                hit.end_voxel.y >= max_chunk_v.y ||
                hit.end_voxel.z >= max_chunk_v.z {
                // did we hit an interesting voxel, or the edge of our search?
                // if so, we're done.
                return hit;
            }
            // we hit the border of the chunk
            cur_coord_v = hit.end;
            cur_voxel_v = hit.end_voxel;
            // go again, look for more chunks
        } else {
            // we're outside of loaded chunks
            let cur_voxel_c = canonicalize_chunk(cur_voxel_v) / SIZE_I;
            let cur_coord_c = to_chunk(cur_coord_v);

            let hit_c = raycast(
                cur_voxel_c,
                cur_coord_c,
                direction,
                min_chunk_c,
                max_chunk_c,
                |v| tracker.get_chunk(storage, v * SIZE_I).is_some()
            );
            cur_coord_v = from_chunk(hit_c.end);

            // finicky: have to recover integer voxel from coordinate hit
            // algorithm: take 
            cur_voxel_v = canonicalize(cur_coord_v);
            if canonicalize(hit_c.end) != hit_c.end_voxel {
                // if there's error, move cur_voxel by 1 in each direction it needs to go.
                let err = hit_c.end_voxel - canonicalize(hit_c.end);
                assert!(err.x.abs() <= 1 && err.y.abs() <= 1 && err.z.abs() <= 1);
                cur_voxel_v += err;
                assert!(canonicalize_chunk(cur_voxel_v) == hit_c.end_voxel * SIZE_I);
            }

            if !hit_c.hit_interesting {
                return Raycast {
                    end: cur_coord_v,
                    end_voxel: cur_voxel_v,
                    ..hit_c
                }
            }
        }
    }
}

// used by voxel_raycast:
// we use a bespoke coordinate system for this operation, since
// `raycast` always uses a grid size of 1, with edges at .5. 

const OFFSET_F: f32 = SIZE_F / 2.0 - 0.5;
const OFFSET: Coord = Coord { x: OFFSET_F, y: OFFSET_F, z: OFFSET_F};

#[inline]
fn to_chunk(voxel_coord: Coord) -> Coord {
    (voxel_coord - OFFSET) / SIZE_F
}
#[inline]
fn from_chunk(chunk_coord: Coord) -> Coord {
    SIZE_F * chunk_coord + OFFSET
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: VoxelCoord = VoxelCoord {
        x: -20,
        y: -20,
        z: -20,
    };
    const MAX: VoxelCoord = VoxelCoord {
        x: 20,
        y: 20,
        z: 20,
    };

    #[test]
    fn raycast_basic() {
        let start = Coord::new(0.0, 0.0, 0.0);
        let target = VoxelCoord::new(5, 10, 15);
        let dir = target.cast().unwrap();
        let hit = raycast(start.cast().unwrap(), start, dir, MIN, MAX, |v| v == target);
        assert_eq!(hit.end_voxel, target);
    }

    #[test]
    fn raycast_edge() {
        let hit = raycast(
            VoxelCoord::new(0, 0, 0),
            Coord::new(0.0, 0.0, 0.0),
            Coord::new(1.0, 0.0, 0.0),
            MIN,
            MAX,
            |_| false,
        );
        assert_eq!(hit.end_voxel, VoxelCoord::new(20, 0, 0));
    }
}
