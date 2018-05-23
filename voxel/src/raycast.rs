//! This module implements the fast voxel traversal algorithm from:
//! "A Fast Voxel Traversal Algorithm for Ray Tracing", John Amanatides, Andrew Woo, 1987
//! http://citeseerx.ist.psu.edu/viewdoc/download?doi=10.1.1.42.3443&rep=rep1&type=pdf

use std::f32;

use super::{canonicalize, Coord, VoxelCoord};

/// Starting at the voxel containing "start", walk through grid squares
/// until the current voxel goes outside the cube defined by:
///  (min.x, max.x) x (min.y, max.y) x (min.z, max.z)
/// (that is, inclusive)
/// if `start` is at the edge of a voxel, the starting voxel will be chosen
/// from one of the voxels on either side of that edge.
///
/// f should return "true" to signal that a voxel has been found.
///
/// Returns a multiple of the direction vector that puts it in the target voxel,
/// and the coordinate of the voxel that occluded the ray.
pub fn raycast<F: FnMut(VoxelCoord) -> bool>(
    start: Coord,
    direction: Coord,
    min: VoxelCoord,
    max: VoxelCoord,
    mut f: F,
) -> (f32, VoxelCoord) {
    // if we're in a target block, return immediately
    let start_voxel = canonicalize(start);
    if f(start_voxel) {
        return (0.0, start_voxel);
    }

    // integer coordinates of the center of our voxel
    let VoxelCoord {
        mut x,
        mut y,
        mut z,
    } = start_voxel;
    let step = canonicalize(direction);
    let (step_x, step_y, step_z) = (step.x.signum(), step.y.signum(), step.z.signum());

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
    let (mut t_max_x, t_dx) = init(start.x, dx);
    let (mut t_max_y, t_dy) = init(start.y, dy);
    let (mut t_max_z, t_dz) = init(start.z, dz);

    for _ in 0..100000 {
        if t_max_x <= t_max_y && t_max_x <= t_max_z {
            x += step_x;
            let cur = VoxelCoord { x, y, z };
            if x == lim_x || f(cur) {
                return (t_max_x, cur);
            }
            t_max_x += t_dx;
        } else if t_max_y < t_max_x && t_max_y <= t_max_z {
            y += step_y;
            let cur = VoxelCoord { x, y, z };
            if y == lim_y || f(cur) {
                return (t_max_y, cur);
            }
            t_max_y += t_dy;
        } else {
            z += step_z;
            let cur = VoxelCoord { x, y, z };
            if z == lim_z || f(cur) {
                return (t_max_z, cur);
            }
            t_max_z += t_dz;
        }
    }
    panic!("nothing hit");
}

fn init(c: f32, dc: f32) -> (f32, f32) {
    let max_c = if dc > 0.0 {
        (c + 1.0).floor()
    } else {
        (c - 1.0).ceil()
    };
    let mut t_max_c = (max_c - c) / dc;
    if t_max_c < 0.0 {
        t_max_c = f32::INFINITY;
    }
    let t_dc = 1.0 / dc;
    (t_max_c, t_dc)
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
        let (mult, hit) = raycast(start, dir, MIN, MAX, |v| v == target);
        assert_eq!(hit, target);
        assert!((mult - 1.0).abs() < 0.001)
    }

    #[test]
    fn raycast_edge() {
        let (_, hit) = raycast(
            Coord::new(0.0, 0.0, 0.0),
            Coord::new(1.0, 0.0, 0.0),
            MIN,
            MAX,
            |_| false,
        );
        assert_eq!(hit, VoxelCoord::new(20, 0, 0));
    }
}
