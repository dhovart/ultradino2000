pub mod game;
pub mod splash;

use bevy::{
    prelude::*,
    reflect::TypeUuid,
    render::{
        camera::{Camera, RenderTarget},
        render_resource::{
            AsBindGroup, Extent3d, ShaderRef, TextureDescriptor, TextureDimension, TextureFormat,
            TextureUsages,
        },
        view::RenderLayers,
    },
    sprite::{Material2d, Material2dPlugin, MaterialMesh2dBundle},
};
use bevy_prototype_lyon::prelude::*;

#[derive(Component)]
struct MainCamera;

// Enum that will be used as a global state for the game
#[derive(Clone, Copy, Eq, PartialEq, Debug, Hash)]
enum GameState {
    Splash,
    Game,
}

#[derive(AsBindGroup, TypeUuid, Clone)]
#[uuid = "b17e3ec0-b8e2-4b66-a62e-1ed9f4374350"]
struct PostProcessingMaterial {
    #[texture(0)]
    #[sampler(1)]
    source_image: Handle<Image>,

    #[uniform(2)]
    pixel_block_size: f32,

    #[uniform(3)]
    chromatic_aberration_intensity: f32,
}

impl Material2d for PostProcessingMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/post_processing.wgsl".into()
    }
}

fn setup(
    mut commands: Commands,
    windows: Res<Windows>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut post_processing_materials: ResMut<Assets<PostProcessingMaterial>>,
    asset_server: Res<AssetServer>,
) {
    asset_server.asset_io().watch_for_changes().unwrap();
    let window = windows.get_primary().unwrap();

    let size = Extent3d {
        width: window.physical_width(),
        height: window.physical_height(),
        ..default()
    };

    // This is the texture that will be rendered to.
    let mut image = Image {
        texture_descriptor: TextureDescriptor {
            label: None,
            size,
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT,
        },
        ..default()
    };

    // fill image.data with zeroes
    image.resize(size);

    let image_handle = images.add(image);
    let post_processing_pass_layer = RenderLayers::layer((RenderLayers::TOTAL_LAYERS - 1) as u8);

    commands.spawn((
        Camera2dBundle {
            projection: OrthographicProjection {
                scale: 0.1,
                ..Default::default()
            },
            camera: Camera {
                target: RenderTarget::Image(image_handle.clone()),
                priority: 0,
                ..default()
            },
            ..Default::default()
        },
        MainCamera,
    ));

    let material_handle = post_processing_materials.add(PostProcessingMaterial {
        source_image: image_handle,
        pixel_block_size: 1.5,
        chromatic_aberration_intensity: 0.002,
    });

    let quad_handle = meshes.add(Mesh::from(shape::Quad::new(Vec2::new(
        size.width as f32,
        size.height as f32,
    ))));

    commands.spawn((
        MaterialMesh2dBundle {
            mesh: quad_handle.into(),
            material: material_handle,
            transform: Transform {
                translation: Vec3::new(0.0, 0.0, 1.5),
                ..default()
            },
            ..default()
        },
        post_processing_pass_layer,
    ));

    // The post-processing pass camera.
    commands.spawn((
        Camera2dBundle {
            camera: Camera {
                // renders after the first main camera
                priority: 1,
                ..default()
            },
            ..Camera2dBundle::default()
        },
        post_processing_pass_layer,
    ));
}

fn main() {
    App::new()
        .insert_resource(Msaa { samples: 1 })
        .insert_resource(ClearColor(Color::hex("333333").unwrap()))
        .add_plugins(DefaultPlugins)
        .add_plugin(Material2dPlugin::<PostProcessingMaterial>::default())
        .add_plugin(ShapePlugin)
        .add_startup_system(setup)
        .add_system(bevy::window::close_on_esc)
        .add_state(GameState::Splash)
        .add_plugin(game::GamePlugin)
        .add_plugin(splash::SplashPlugin)
        .run();
}

fn despawn_screen<T: Component>(to_despawn: Query<Entity, With<T>>, mut commands: Commands) {
    for entity in &to_despawn {
        commands.entity(entity).despawn_recursive();
    }
}
