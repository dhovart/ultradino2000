use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;
use bevy_rapier2d::prelude::*;
use rand::seq::SliceRandom;
use rand::{thread_rng, Rng};
use std::cmp;

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
        .add_system(controls)
        .add_system(jetpack)
        .add_system(particles)
        .add_system(laser_eyes)
        .add_system(lasers)
        .run();
}

#[derive(Component)]
struct Player;

enum Direction {
    LEFT,
    RIGHT,
}

#[derive(Component)]
struct IsShooting(bool);

#[derive(Component, Default)]
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

fn jetpack(
    mut commands: Commands,
    query: Query<(&Controls, &RigidBodyPositionComponent), With<Player>>,
) {
    let (controls, rb_pos) = query.single();
    if controls.propulsion {
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
                            + angle.cos() * (offset + rng.gen_range(-0.7..0.7))
                            + particle_direction.x * 3.8,
                        center.y
                            + angle.sin() * (offset + rng.gen_range(-0.7..0.7))
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
                    .spawn()
                    .insert(LaserRay::new(position))
                    .insert_bundle(GeometryBuilder::build_as(
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
    )>,
    player_query: Query<(&Controls, &RigidBodyPositionComponent), With<Player>>,
) {
    let (controls, rb_pos) = player_query.single();

    let angle = rb_pos.position.rotation.angle();
    let direction = Vec2::new(-angle.sin(), angle.cos());
    let center = rb_pos.position.translation.vector;

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
                center.x + angle.cos() * offset + direction.x * 5.,
                center.y + angle.sin() * offset + direction.y * 5.,
                11.,
            );

            if let DrawMode::Stroke(stroke_mode) = *mode {
                let color = stroke_mode.color;
                let min_width = 0.1;
                let width = stroke_mode.options.line_width;
                let width = width + (time.seconds_since_startup() as f32 * 60.).sin() * 0.1;
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
    mut query: Query<
        (
            &Controls,
            &RigidBodyPositionComponent,
            &RigidBodyMassPropsComponent,
            &mut RigidBodyVelocityComponent,
        ),
        With<Player>,
    >,
) {
    let (controls, rb_pos, rb_mprops, mut rb_vel) = query.single_mut();
    rb_vel.angvel = match controls.rotation {
        Some(Direction::LEFT) => 5.,
        Some(Direction::RIGHT) => -5.,
        None => 0.0,
    };
    rb_vel.linvel *= 0.99;
    let angle = rb_pos.position.rotation.angle();
    if controls.propulsion {
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
        .insert(IsShooting(false))
        .insert(Controls::default())
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
