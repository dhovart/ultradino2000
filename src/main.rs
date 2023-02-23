use bevy::prelude::*;
use bevy_prototype_lyon::prelude::FillMode;
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
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(10.))
        .add_plugin(RapierDebugRenderPlugin::default())
        .insert_resource(ClearColor(Color::hex("1d1d1d").unwrap()))
        .insert_resource(RapierConfiguration {
            gravity: Vec2::new(0., 0.),
            ..Default::default()
        })
        .add_startup_system(setup)
        .add_startup_stage("game_setup_actors", SystemStage::single(player_spawn))
        .add_system(player)
        .add_system(controls)
        .add_system(jetpack)
        .add_system(particles)
        .add_system(laser_eyes)
        .add_system(lasers)
        .run();
}

#[derive(Component)]
struct Player;

#[derive(Debug)]
enum Direction {
    LEFT,
    RIGHT,
}

#[derive(Component)]
struct IsShooting(bool);

#[derive(Component, Default, Debug)]
struct Controls {
    rotation: Option<Direction>,
    propulsion: bool,
    shooting: bool,
}

const JETPACK_PARTICLE_COLORS: [&'static str; 3] = ["fff200", "ed1c24", "ff7f27"];
const JETPACK_PARTICLE_LIFETIME: i32 = 20;

#[derive(Component)]
struct Particle {
    lifetime: i32,
    direction: Vec2,
}

impl Particle {
    fn new(direction: Vec2, lifetime: i32) -> Self {
        Particle {
            lifetime,
            direction,
        }
    }
}

#[derive(Component)]
struct LaserRay {
    height: f32,
    position: Direction,
}

impl LaserRay {
    fn new(position: Direction) -> Self {
        LaserRay {
            height: 0.1,
            position,
        }
    }
}
fn jetpack(mut commands: Commands, query: Query<(&Controls, &Transform), With<Player>>) {
    let (controls, rb_transform) = query.single();
    if controls.propulsion {
        let rotation_as_vector = rb_transform.rotation.mul_vec3(Vec3::new(0.0, 1.0, 0.0));
        let particle_direction = Vec2::new(rotation_as_vector.x, rotation_as_vector.y) * -1.;
        let center = rb_transform.translation;

        let mut spawn_particle = |offset: f32| {
            let shape = shapes::Rectangle {
                extents: Vec2::new(0.7, 0.7),
                origin: shapes::RectangleOrigin::Center,
            };
            let mut rng = thread_rng();
            let color = Color::hex(JETPACK_PARTICLE_COLORS.choose(&mut rng).unwrap()).unwrap();
            commands
                .spawn_empty()
                .insert(Particle::new(
                    particle_direction.into(),
                    JETPACK_PARTICLE_LIFETIME,
                ))
                .insert(GeometryBuilder::build_as(
                    &shape,
                    DrawMode::Fill(FillMode::color(color)),
                    Transform::from_translation(Vec3::new(
                        center.x
                            + rotation_as_vector.x.cos() * (offset + rng.gen_range(-0.7..0.7))
                            + particle_direction.x * 3.8,
                        center.y
                            + rotation_as_vector.y.cos() * (offset + rng.gen_range(-0.7..0.7))
                            + particle_direction.y * 3.8,
                        10.,
                    )),
                ));
        };

        spawn_particle(0.8);
        spawn_particle(-0.8);
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

fn laser_eyes(
    mut commands: Commands,
    mut query: Query<(&Controls, &mut IsShooting), With<Player>>,
) {
    let (controls, mut is_shooting) = query.single_mut();
    if let IsShooting(false) = *is_shooting {
        if controls.shooting {
            *is_shooting = IsShooting(true);

            let color = Color::hex("ed1c24").unwrap();
            let line = shapes::Line(Vec2::ZERO, Vec2::ZERO);
            let mut spawn_laser = |position: Direction| {
                commands
                    .spawn_empty()
                    .insert(LaserRay::new(position))
                    .insert(GeometryBuilder::build_as(
                        &line,
                        DrawMode::Stroke(StrokeMode::new(color, 0.1)),
                        Transform::default(),
                    ));
            };
            spawn_laser(Direction::LEFT);
            spawn_laser(Direction::RIGHT);
        }
    } else {
        *is_shooting = IsShooting(controls.shooting);
    }
}

fn lasers(
    time: Res<Time>,
    mut commands: Commands,
    mut ray_query: Query<(
        Entity,
        &mut Transform,
        &mut Path,
        &mut DrawMode,
        &mut LaserRay,
    ), Without<Player>>,
    player_query: Query<(&Controls, &Transform), With<Player>>,
) {
    let (controls, rb_transform) = player_query.single();

    let rotation_as_vector = rb_transform.rotation.mul_vec3(Vec3::new(0.0, 1.0, 0.0));
    let direction = Vec2::new(rotation_as_vector.x, rotation_as_vector.y);
    let center = rb_transform.translation;

    for (entity, mut transform, mut path, mut mode, mut laser) in ray_query.iter_mut() {
        if !controls.shooting {
            commands.entity(entity).despawn();
        } else {
            let offset = match laser.position {
                Direction::LEFT => -0.22,
                Direction::RIGHT => 0.28,
            };

            laser.height += 150. * time.delta_seconds();
            let line = shapes::Line(Vec2::ZERO, direction * laser.height);
            *path = ShapePath::build_as(&line);
            transform.translation = Vec3::new(
                center.x + rotation_as_vector.x.cos() * offset + direction.x * 5.,
                center.y + rotation_as_vector.y.sin() * offset + direction.y * 5.,
                11.,
            );

            if let DrawMode::Stroke(stroke_mode) = *mode {
                let color = stroke_mode.color;
                let min_width = 0.1;
                let width = stroke_mode.options.line_width;
                let width = width + (time.elapsed_seconds() * 60.).sin() * 0.1;
                let width = width.max(min_width);
                *mode = DrawMode::Stroke(StrokeMode::new(color, width));
            }
        }

        // TODO: check for collisions casting a ray
    }
}

fn controls(input: Res<Input<KeyCode>>, mut query: Query<&mut Controls, With<Player>>) {
    let mut controls = query.single_mut();
    controls.rotation = if input.pressed(KeyCode::Left) {
        Some(Direction::LEFT)
    } else if input.pressed(KeyCode::Right) {
        Some(Direction::RIGHT)
    } else {
        None
    };
    controls.propulsion = input.pressed(KeyCode::Up);
    controls.shooting = input.pressed(KeyCode::Space);
}

fn player(
    mut query: Query<(&Controls, &Transform, &mut Velocity, &mut ExternalImpulse), With<Player>>,
) {
    let (controls, transform, mut velocity, mut impulse) = query.single_mut();
    velocity.angvel = match controls.rotation {
        Some(Direction::LEFT) => 5.,
        Some(Direction::RIGHT) => -5.,
        None => 0.0,
    };

    let vector = transform.rotation.mul_vec3(Vec3::new(0.0, 1.0, 0.0));
    let direction = Vec2::new(vector.x, vector.y);
    if controls.propulsion {
        impulse.impulse = direction * 0.5;
    }
}

fn player_spawn(mut commands: Commands, asset_server: Res<AssetServer>) {
    let half_body_width = 110. * PIXEL_TO_METERS;
    let half_body_height = 280. * PIXEL_TO_METERS;

    let collider = Collider::cuboid(half_body_width, half_body_height);

    let transform = Transform::from_xyz(0.0, half_body_height, 0.);

    let mut prev_id = commands
        .spawn(RigidBody::Dynamic)
        .insert(collider)
        .insert(TransformBundle::from(transform))
        .insert(Velocity::default())
        .insert(ExternalImpulse::default())
        .insert(SpriteBundle {
            texture: asset_server.load("dino.png"),
            sprite: Sprite {
                custom_size: Some(Vec2::new(half_body_width * 2., half_body_height * 2.)),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Player)
        .insert(IsShooting(false))
        .insert(Controls::default())
        .id();

    let mut pos_y = -half_body_height - 0.7;
    let mut prev_half_height = half_body_height;
    let mut half_width = 0.5;

    for _ in 0..8 {
        let half_height = half_width * 1.2;

        let collider = Collider::cuboid(half_width, half_height);

        let shape = shapes::Rectangle {
            extents: Vec2::new(half_width * 2., half_height * 2.),
            origin: shapes::RectangleOrigin::Center,
        };

        let joint = RevoluteJointBuilder::new()
            .local_anchor1(Vec2::new(0.0, -prev_half_height))
            .local_anchor2(Vec2::new(0.0, half_height))
            .limits([-10., 10.]);

        let id = commands
            .spawn(RigidBody::Dynamic)
            .insert(collider)
            .insert(GeometryBuilder::build_as(
                &shape,
                DrawMode::Fill(FillMode::color(Color::hex("26b24a").unwrap())),
                Transform::from_xyz(0., pos_y, 0.),
            ))
            .insert(ImpulseJoint::new(prev_id, joint))
            .id();

        pos_y -= half_height * 2.;
        half_width -= 0.05;
        prev_id = id;
        prev_half_height = half_height;
    }
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle {
        projection: OrthographicProjection {
            scale: 0.1,
            ..Default::default()
        },
        ..Default::default()
    });
}
