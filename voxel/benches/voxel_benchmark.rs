#[macro_use]
extern crate criterion;

extern crate voxel;

use criterion::Criterion;

use voxel::{TestVoxel, Chunk, VoxelCoord, Coord, CHUNK_SIZE};
use voxel::mesh::{InProgress, Direction, mesh_layer};
use voxel::raycast::raycast;

fn mesh() {
    let mut chunk = Chunk::<TestVoxel>::empty(VoxelCoord::new(0, 0, 0));

    for i in 0..CHUNK_SIZE {
        chunk.voxels[i][i][i] = TestVoxel::Air;
        chunk.voxels[i][CHUNK_SIZE-i-1][i] = TestVoxel::Air;
        chunk.voxels[CHUNK_SIZE-i-1][i][i] = TestVoxel::Air;
        chunk.voxels[i][i][CHUNK_SIZE-i-1] = TestVoxel::Air;
    }

    let mut in_progress = InProgress {
        color: vec![],
        position: vec![],
        normal: vec![],
    };

    let directions = [
        (0, CHUNK_SIZE as i16 - 1,  1, Direction::East),
        (0, CHUNK_SIZE as i16 - 1,  1, Direction::Up),
        (0, CHUNK_SIZE as i16 - 1,  1, Direction::North),
        (1, CHUNK_SIZE as i16    , -1, Direction::West),
        (1, CHUNK_SIZE as i16    , -1, Direction::Down),
        (1, CHUNK_SIZE as i16    , -1, Direction::South)
    ];

    //println!("");
    for (start, end, sub, direction) in directions.into_iter() {
        //println!("{} {} {} {:?}", start, end, sub, normal);
        for i in *start..*end {
            mesh_layer(&chunk, i, &chunk, i+sub, *direction, &mut in_progress)
        }
    }
}

fn raycast_simple_16() {
    criterion::black_box(raycast(
        Coord::new(0.0, 0.0, 0.0),
        Coord::new(0.37, 0.299, 0.936),
        VoxelCoord::new(-16, -16, -16),
        VoxelCoord::new(16, 16, 16),
        |_| false
    ));
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("mesh 1", |b| b.iter(|| mesh()));
    c.bench_function("raycast_simple_16 1", |b| b.iter(|| raycast_simple_16()));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);