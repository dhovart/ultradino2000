use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;
use bevy_rapier2d::prelude::*;
use rand::seq::SliceRandom;
use rand::{thread_rng, Rng};

// FIXME: refactor in bevy plugins

const PIXEL_TO_METERS: f32 = 0.02;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(ShapePlugin)
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugin(RapierRenderPlugin)
        .insert_resource(ClearColor(Color::hex("1d1d1d").unwrap()))
        .insert_resource(RapierConfiguration {
            gravity: Vector::new(0., 0.),
            ..Default::default()
        })
        .add_startup_system(setup)
        .add_startup_stage("game_setup_actors", SystemStage::single(player_spawn))
        .add_system(player)
        .add_system(movement)
        .add_system(jetpack)
        .add_system(particles)
        .run();
}

#[derive(Component)]
struct Player;

enum Rotation {
    LEFT,
    RIGHT,
}

#[derive(Component)]
struct Movement {
    rotation: Option<Rotation>,
    propulsion: bool,
}

impl Default for Movement {
    fn default() -> Self {
        Movement {
            rotation: None,
            propulsion: false,
        }
    }
}

#[derive(Component)]
struct Particle {
    lifetime: i32,
    direction: Vec2,
}

const JETPACK_PARTICLE_COLORS: [&'static str; 3] = ["fff200", "ed1c24", "ff7f27"];
const JETPACK_PARTICLE_LIFETIME: i32 = 20;

impl Particle {
    fn new(direction: Vec2, lifetime: i32) -> Self {
        Particle {
            lifetime,
            direction,
        }
    }
}

fn jetpack(
    mut commands: Commands,
    query: Query<(&Movement, &RigidBodyPositionComponent), With<Player>>,
) {
    let (movement, rb_pos) = query.single();
    if movement.propulsion {
        let angle = rb_pos.position.rotation.angle();
        let particle_direction = Vec2::new(angle.sin(), -angle.cos());
        let center = rb_pos.position.translation.vector;

        let mut spawn_particle = |offset: f32| {
            let shape = shapes::Rectangle {
                extents: Vec2::new(0.7, 0.7),
                origin: shapes::RectangleOrigin::Center,
            };
            let mut rng = thread_rng();
            let color = Color::hex(JETPACK_PARTICLE_COLORS.choose(&mut rng).unwrap()).unwrap();
            commands
                .spawn()
                .insert(Particle::new(
                    particle_direction.into(),
                    JETPACK_PARTICLE_LIFETIME,
                ))
                .insert_bundle(GeometryBuilder::build_as(
                    &shape,
                    DrawMode::Fill(FillMode::color(color)),
                    Transform::from_translation(Vec3::new(
                        center.x
                            + angle.cos() * offset
                            + rng.gen_range(-0.4..0.4)
                            + particle_direction.x * 4.,
                        center.y
                            + angle.sin() * offset
                            + rng.gen_range(-0.4..0.4)
                            + particle_direction.y * 4.,
                        10.,
                    )),
                ));
        };

        spawn_particle(1.);
        spawn_particle(-1.);
    }
}

fn particles(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &mut Particle, &mut DrawMode)>,
) {
    for (entity, mut transform, mut particle, mut mode) in query.iter_mut() {
        particle.lifetime -= 1;
        if particle.lifetime == 0 {
            commands.entity(entity).despawn();
        } else {
            let translation = &mut transform.translation;
            translation.x += particle.direction.x * time.delta_seconds() * 10.;
            translation.y += particle.direction.y * time.delta_seconds() * 10.;

            if let DrawMode::Fill(fill_mode) = *mode {
                let mut color = fill_mode.color;
                color.set_a(particle.lifetime as f32 / JETPACK_PARTICLE_LIFETIME as f32);
                *mode = DrawMode::Fill(FillMode::color(color));
            }
        }
    }
}

fn movement(input: Res<Input<KeyCode>>, mut query: Query<&mut Movement, With<Player>>) {
    let mut movement = query.single_mut();
    movement.rotation = if input.pressed(KeyCode::Left) {
        Some(Rotation::LEFT)
    } else if input.pressed(KeyCode::Right) {
        Some(Rotation::RIGHT)
    } else {
        None
    };
    movement.propulsion = input.pressed(KeyCode::Up);
}

fn player(
    mut query: Query<
        (
            &Movement,
            &RigidBodyPositionComponent,
            &RigidBodyMassPropsComponent,
            &mut RigidBodyVelocityComponent,
        ),
        With<Player>,
    >,
) {
    let (movement, rb_pos, rb_mprops, mut rb_vel) = query.single_mut();
    rb_vel.angvel = match movement.rotation {
        Some(Rotation::LEFT) => 5.,
        Some(Rotation::RIGHT) => -5.,
        None => 0.0,
    };
    rb_vel.linvel *= 0.99;
    let angle = rb_pos.position.rotation.angle();
    if movement.propulsion {
        let impulse = Vec2::new(-angle.sin() * 20., angle.cos() * 20.);
        rb_vel.apply_impulse(rb_mprops, impulse.into());
    }
}

fn player_spawn(mut commands: Commands, asset_server: Res<AssetServer>) {
    let body_width = 110. * PIXEL_TO_METERS;
    let body_height = 280. * PIXEL_TO_METERS;

    let collider = ColliderBundle {
        shape: ColliderShape::cuboid(body_width, body_height).into(),
        ..Default::default()
    };

    let rigid_body = RigidBodyBundle {
        position: Vec2::new(0.0, 20. + body_height).into(),
        ..Default::default()
    };

    let mut pos = Vec2::new(0.0, 20.0);
    let mut prev_half_height = body_height;
    let mut half_width = 0.5;

    let mut prev_id = commands
        .spawn_bundle(rigid_body)
        .insert_bundle(collider)
        .insert_bundle(SpriteBundle {
            texture: asset_server.load("dino.png"),
            sprite: Sprite {
                custom_size: Some(Vec2::new(body_width * 2., body_height * 2.)),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Player)
        .insert(Movement::default())
        .insert(ColliderPositionSync::Discrete)
        .id();

    for _ in 0..8 {
        let half_height = half_width * 1.2;

        let collider = ColliderBundle {
            shape: ColliderShape::cuboid(half_width, half_height).into(),
            ..Default::default()
        };

        let rigid_body = RigidBodyBundle {
            position: pos.into(),
            ..Default::default()
        };

        let shape = shapes::Rectangle {
            extents: Vec2::new(half_width * 2., half_height * 2.),
            origin: shapes::RectangleOrigin::Center,
        };

        let id = commands
            .spawn_bundle(rigid_body)
            .insert_bundle(collider)
            .insert(ColliderPositionSync::Discrete)
            .insert_bundle(GeometryBuilder::build_as(
                &shape,
                DrawMode::Fill(FillMode::color(Color::hex("26b24a").unwrap())),
                Transform::default(),
            ))
            .id();

        commands.spawn_bundle((JointBuilderComponent::new(
            RevoluteJoint::new()
                .local_anchor1(point![0.0, -prev_half_height])
                .local_anchor2(point![0.0, half_height])
                .limit_axis([-10., 10.]),
            prev_id,
            id,
        ),));

        pos -= Vec2::new(0., half_height * 2.);
        half_width -= 0.05;
        prev_id = id;
        prev_half_height = half_height;
    }
}

fn setup(mut commands: Commands) {
    let mut camera_bundle = OrthographicCameraBundle::new_2d();
    camera_bundle.orthographic_projection.scale = 0.1;
    commands.spawn_bundle(camera_bundle);
}
