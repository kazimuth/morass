//! Displays a shaded sphere to the user.

extern crate amethyst;
extern crate morass_voxel;

use morass_voxel::{MorassVoxel, MorassChunk, VoxelCoord};

use std::time::Duration;

use amethyst::core::cgmath::Deg;
use amethyst::core::transform::GlobalTransform;
use amethyst::ecs::prelude::World;
use amethyst::prelude::*;
use amethyst::renderer::{AmbientColor, Camera, DisplayConfig, DrawShadedSeparate, Event, KeyboardInput,
                         Light, Pipeline, PointLight, Projection, RenderBundle,
                         Rgba, Stage, VirtualKeyCode, WindowEvent};

const AMBIENT_LIGHT_COLOUR: Rgba = Rgba(0.1, 0.1, 0.1, 1.0); // near-black
const POINT_LIGHT_COLOUR: Rgba = Rgba(1.0, 1.0, 1.0, 1.0); // white
const BACKGROUND_COLOUR: [f32; 4] = [0.0, 0.0, 0.0, 0.0]; // black
const LIGHT_POSITION: [f32; 3] = [20.0, 20.0, -20.0];
const LIGHT_RADIUS: f32 = 50.0;
const LIGHT_INTENSITY: f32 = 3.0;

struct Example;

impl<'a, 'b> State<GameData<'a, 'b>> for Example {
    fn on_start(&mut self, data: StateData<GameData>) {
        // Initialise the scene with an object, a light and a camera.
        initialise_lights(data.world);
        initialise_camera(data.world);
        initialize_voxels(data.world);
    }

    fn handle_event(&mut self, _: StateData<GameData>, event: Event) -> Trans<GameData<'a, 'b>> {
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(VirtualKeyCode::Escape),
                            ..
                        },
                    ..
                } => Trans::Quit,
                _ => Trans::None,
            },
            _ => Trans::None,
        }
    }

    fn update(&mut self, data: StateData<GameData>) -> Trans<GameData<'a, 'b>> {
        data.data.update(&data.world);
        Trans::None
    }
}

fn run() -> Result<(), amethyst::Error> {
    let display_config_path = format!(
        "{}/examples/sphere/resources/display_config.ron",
        env!("CARGO_MANIFEST_DIR")
    );

    let resources = format!("{}/examples/assets/", env!("CARGO_MANIFEST_DIR"));

    let pipe = Pipeline::build().with_stage(
        Stage::with_backbuffer()
            .clear_target(BACKGROUND_COLOUR, 1.0)
            .with_pass(DrawShadedSeparate::new()),
            //.with_pass(DrawShaded::<PosNormTex>::new()),
    );

    let config = DisplayConfig::load(&display_config_path);

    let game_data = GameDataBuilder::default()
        .with_bundle(RenderBundle::new(pipe, Some(config)))?
        .with(morass_voxel::tracker::ChunkTrackerSystem::<MorassVoxel>::new(), "chunk_tracker", &[])
        .with(morass_voxel::mesh::ChunkMesherSystem::<MorassVoxel>::new(Duration::from_millis(3)), "chunk_mesher", &[]);
    let mut game = Application::new(resources, Example, game_data)?;
    game.run();
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        println!("Failed to execute example: {}", e);
        ::std::process::exit(1);
    }
}

fn initialize_voxels(world: &mut World) {
    world.add_resource(morass_voxel::tracker::ChunkTracker::new());
    let mut chunk = MorassChunk::empty(VoxelCoord::new(0,0,0));
    chunk.voxels[0][0][0] = MorassVoxel::Grass;
    chunk.voxels[0][0][2] = MorassVoxel::Grass;
    chunk.voxels[0][0][1] = MorassVoxel::Grass;
    chunk.voxels[0][0][0] = MorassVoxel::Grass;
    chunk.voxels[0][3][3] = MorassVoxel::Grass;
    chunk.voxels[5][0][5] = MorassVoxel::Grass;
    chunk.voxels[0][5][0] = MorassVoxel::Grass;
    use amethyst::core::cgmath::Matrix4;
    world.create_entity()
         .with(chunk)
         .with(GlobalTransform(Matrix4::from_translation([0.0, 0.0, 0.0].into())))
         .build();

}

/// This function adds an ambient light and a point light to the world.
fn initialise_lights(world: &mut World) {
    // Add ambient light.
    world.add_resource(AmbientColor(AMBIENT_LIGHT_COLOUR));

    let light: Light = PointLight {
        center: LIGHT_POSITION.into(),
        radius: LIGHT_RADIUS,
        intensity: LIGHT_INTENSITY,
        color: POINT_LIGHT_COLOUR,
        ..Default::default()
    }.into();

    // Add point light.
    world.create_entity().with(light).build();
}

/// This function initialises a camera and adds it to the world.
fn initialise_camera(world: &mut World) {
    use amethyst::core::cgmath::Matrix4;
    let transform =
        Matrix4::from_translation([0.0, 0.0, -20.0].into()) * Matrix4::from_angle_y(Deg(180.));
    world
        .create_entity()
        .with(Camera::from(Projection::perspective(1.3, Deg(60.0))))
        .with(GlobalTransform(transform.into()))
        .build();
}