use super::{despawn_screen, GameState, Transition};


use bevy::{
    input::keyboard::KeyboardInput,
    prelude::*,
    reflect::TypeUuid,
    render::render_resource::{AsBindGroup, ShaderRef},
    sprite::{Material2d, Material2dPlugin, MaterialMesh2dBundle},
};

#[derive(AsBindGroup, TypeUuid, Debug, Clone)]
#[uuid = "f690fdae-d598-45ab-8225-97e2a3f056e0"]
pub struct SplashMaterial {
    #[texture(0)]
    #[sampler(1)]
    texture: Handle<Image>,

    #[uniform(2)]
    time: f32,
}

impl Material2d for SplashMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/splash.wgsl".into()
    }
}

pub struct SplashPlugin;

impl Plugin for SplashPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(Material2dPlugin::<SplashMaterial>::default())
            .add_system_set(SystemSet::on_enter(GameState::Splash).with_system(setup_splash))
            .add_system_set(SystemSet::on_update(GameState::Splash).with_system(update))
            .add_system_set(
                SystemSet::on_exit(GameState::Splash).with_system(despawn_screen::<OnSplashScreen>),
            );
    }
}

#[derive(Component)]
struct OnSplashScreen;

fn setup_splash(
    mut commands: Commands,
    mut materials: ResMut<Assets<SplashMaterial>>,
    //mut post_processing_materials: ResMut<Assets<PostProcessingMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    asset_server: Res<AssetServer>,
) {
    let texture = asset_server.load("splash.png");
    commands.spawn((
        OnSplashScreen,
        MaterialMesh2dBundle {
            material: materials.add(SplashMaterial {
                time: 0.,
                texture,
            }),
            mesh: meshes
                .add(Mesh::from(shape::Quad {
                    size: Vec2::new(70., 10.),
                    ..Default::default()
                }))
                .into(),
            ..Default::default()
        },
    ));

}

fn update(
    mut key_evr: EventReader<KeyboardInput>,
    mut splash_materials: ResMut<Assets<SplashMaterial>>,
    mut transition: ResMut<Transition>,
    time: Res<Time>,
) {
    let time = time.elapsed().as_secs_f32();
    for (_, material) in splash_materials.iter_mut() {
        material.time = time;
    }

    for ev in key_evr.iter() {
        match ev.state {
            _ => transition.is_playing = true,
        }
    }
}
