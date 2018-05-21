#[macro_use]
extern crate criterion;

extern crate voxel;

use criterion::Criterion;

use voxel::{TestVoxel, Chunk, ChunkCoord, CHUNK_SIZE};
use voxel::mesh::{InProgress, mesh_layer};

fn mesh() {
    let mut chunk = Chunk::<TestVoxel>::empty(ChunkCoord::new(0, 0, 0));

    for i in 0..CHUNK_SIZE {
        chunk.voxels[i][i][i] = TestVoxel::Air;
        chunk.voxels[i][CHUNK_SIZE-i-1][i] = TestVoxel::Air;
        chunk.voxels[CHUNK_SIZE-i-1][i][i] = TestVoxel::Air;
        chunk.voxels[i][i][CHUNK_SIZE-i-1] = TestVoxel::Air;
    }

    let normal = ChunkCoord::unit_z();

    let mut in_progress = InProgress {
        color: vec![],
        position: vec![],
        normal: vec![],
        tangent: vec![]
    };
    let directions = [
        (0, CHUNK_SIZE as i16 - 1,  1, ChunkCoord::new(1,0,0), ChunkCoord::new(1,0,0)),
        (0, CHUNK_SIZE as i16 - 1,  1, ChunkCoord::new(0,1,0), ChunkCoord::new(0,1,0)),
        (0, CHUNK_SIZE as i16 - 1,  1, ChunkCoord::new(0,0,1), ChunkCoord::new(0,0,1)),
        (1, CHUNK_SIZE as i16    , -1, ChunkCoord::new(1,0,0), ChunkCoord::new(-1,0,0)),
        (1, CHUNK_SIZE as i16    , -1, ChunkCoord::new(0,1,0), ChunkCoord::new(0,-1,0)),
        (1, CHUNK_SIZE as i16    , -1, ChunkCoord::new(0,0,1), ChunkCoord::new(0,0,-1))
    ];

    //println!("");
    for (start, end, sub, axis, normal) in directions.into_iter() {
        //println!("{} {} {} {:?}", start, end, sub, normal);
        for i in *start..*end {
            mesh_layer(&chunk, i, &chunk, i+sub, *axis, *normal, &mut in_progress)
        }
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("mesh 1", |b| b.iter(|| mesh()));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);