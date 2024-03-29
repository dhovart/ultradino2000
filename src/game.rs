// FIXME refactor in smaller plugins

use super::{despawn_screen, GameState, Transition};

use bevy::prelude::*;

use bevy_prototype_lyon::{
    prelude::{FillMode, *},
    shapes::Polygon,
};

use bevy_rapier2d::prelude::*;

use rand::seq::SliceRandom;
use rand::{thread_rng, Rng};

const PIXEL_TO_METERS: f32 = 0.02;

#[derive(Component)]
struct Player;

#[derive(Component)]
struct Asteroid;

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
const JETPACK_PARTICLE_LIFETIME: i32 = 30;

#[derive(Component)]
struct Destructible(bool);

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
struct Monster;

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

trait SteeringBehavior {
    fn get_steering(&self) -> Vec2 {
        Vec2::splat(0.)
    }
    fn get_angular_steering(&self) -> f32 {
        0.
    }
}

#[derive(Component)]
struct SeekBehavior {
    target: Vec2,
    position: Vec2,
    max_speed: f32,
}

impl SteeringBehavior for SeekBehavior {
    fn get_steering(&self) -> Vec2 {
        let desired = self.target - self.position;
        desired.normalize() * self.max_speed
    }
}

#[derive(Component)]
struct KeepVerticalBehavior {
    max_speed: f32,
    angle: f32,
}

impl SteeringBehavior for KeepVerticalBehavior {
    fn get_angular_steering(&self) -> f32 {
        if self.angle < 0. {
            self.max_speed
        } else if self.angle > 0. {
            -self.max_speed
        } else {
            0.
        }
    }
}

#[derive(Component)]
struct FleeBehavior {
    target: Vec2,
    position: Vec2,
    max_speed: f32,
}

impl SteeringBehavior for FleeBehavior {
    fn get_steering(&self) -> Vec2 {
        let seek = SeekBehavior {
            target: self.target,
            position: self.position,
            max_speed: self.max_speed,
        };
        seek.get_steering() * -1.
    }
}

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(10.))
            //.add_plugin(RapierDebugRenderPlugin::default())
            .insert_resource(RapierConfiguration {
                gravity: Vec2::new(0., 0.),
                ..Default::default()
            })
            .add_system_set(
                SystemSet::on_enter(GameState::Game)
                    .with_system(transition)
                    .with_system(player_spawn)
                    .with_system(asteroids_spawn)
                    .with_system(monsters_spawn),
            )
            .add_system_set(
                SystemSet::on_update(GameState::Game)
                    .with_system(controls)
                    .with_system(player)
                    .with_system(jetpack)
                    .with_system(particles)
                    .with_system(laser_eyes)
                    .with_system(lasers)
                    .with_system(camera)
                    .with_system(update_monsters_behaviors),
            )
            .add_system_set(
                SystemSet::on_exit(GameState::Game).with_system(despawn_screen::<OnGameScreen>),
            );
    }
}

fn transition(mut transition: ResMut<Transition>) {
    transition.to_state = None;
    transition.step = -0.01;
    transition.is_playing = true;
}

// Tag component used to tag entities added on the game screen
#[derive(Component)]
struct OnGameScreen;

// FIXME make behaviors generic
fn update_monsters_behaviors(
    mut monsters_query: Query<
        (
            &mut ExternalForce,
            &Transform,
            &mut SeekBehavior,
            &mut KeepVerticalBehavior,
        ),
        With<Monster>,
    >,
    player_query: Query<&Transform, With<Player>>,
) {
    let player_transform = player_query.single();
    for (
        mut monster_force,
        monster_transform,
        mut seek_behavior,
        mut keep_vertical_behavior
    ) in
        monsters_query.iter_mut()
    {
        seek_behavior.position = Vec2::new(
            monster_transform.translation.x,
            monster_transform.translation.y,
        );
        seek_behavior.target = Vec2::new(
            player_transform.translation.x,
            player_transform.translation.y,
        );
        keep_vertical_behavior.angle = monster_transform.rotation.z;
        monster_force.torque = keep_vertical_behavior.get_angular_steering();
        monster_force.force = seek_behavior.get_steering();
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
            commands.spawn((
                Particle::new(particle_direction.into(), JETPACK_PARTICLE_LIFETIME),
                OnGameScreen,
                GeometryBuilder::build_as(
                    &shape,
                    DrawMode::Fill(FillMode::color(color)),
                    Transform::from_translation(Vec3::new(
                        center.x
                            + rotation_as_vector.x.cos() * (offset + rng.gen_range(-0.7..0.7))
                            + particle_direction.x * 3.8,
                        center.y
                            + rotation_as_vector.y.cos() * (offset + rng.gen_range(-0.7..0.7))
                            + particle_direction.y * 3.8,
                        0.1,
                    )),
                ),
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
                commands.spawn((
                    LaserRay::new(position),
                    OnGameScreen,
                    GeometryBuilder::build_as(
                        &line,
                        DrawMode::Stroke(StrokeMode::new(color, 0.1)),
                        Transform::default(),
                    ),
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
    rapier_context: Res<RapierContext>,
    mut commands: Commands,
    mut ray_query: Query<
        (
            Entity,
            &mut Transform,
            &mut Path,
            &mut DrawMode,
            &mut LaserRay,
        ),
        With<LaserRay>,
    >,
    player_query: Query<(Entity, &Controls, &Transform), (With<Player>, Without<LaserRay>)>,
    asteroid_query: Query<
        (&Collider, &Transform, &Destructible),
        (With<Asteroid>, Without<LaserRay>),
    >,
) {
    let (player_entity, controls, rb_transform) = player_query.single();

    let rotation_as_vector = rb_transform.rotation.mul_vec3(Vec3::new(0.0, 1.0, 0.0));
    let direction = Vec2::new(rotation_as_vector.x, rotation_as_vector.y);
    let center = rb_transform.translation;

    for (entity, mut transform, mut path, mut mode, mut laser) in ray_query.iter_mut() {
        if !controls.shooting {
            commands.entity(entity).despawn();
        } else {
            laser.height += 600. * time.delta_seconds();
            laser.height = laser.height.min(40.);

            let filter = QueryFilter::default().exclude_collider(player_entity);
            let ray_pos = Vec2::new(transform.translation.x, transform.translation.y);
            let max_toi = laser.height;

            if let Some((entity, toi)) =
                rapier_context.cast_ray(ray_pos, direction, max_toi, true, filter)
            {
                laser.height = laser.height.min(toi);
                let hit_point = ray_pos + direction * toi;
                if let Some((
                    asteroid_collider,
                    &asteroid_transform,
                    Destructible(is_destructible),
                )) = asteroid_query.get(entity).ok()
                {
                    if *is_destructible {
                        let sub_polys = subdivide(&asteroid_collider);
                        for sub_poly in sub_polys {
                            let points: Vec<Vec2> =
                                sub_poly.as_convex_polygon().unwrap().points().collect();

                            let is_destructible = polygon_area(&points) >= 8.;

                            let shape = shapes::Polygon::from(Polygon {
                                points: points,
                                closed: true,
                            });

                            let mut rng = thread_rng();
                            let entity = commands
                                .spawn((
                                    Asteroid,
                                    OnGameScreen,
                                    RigidBody::Dynamic,
                                    sub_poly,
                                    Destructible(is_destructible),
                                    ExternalImpulse {
                                        torque_impulse: rng.gen_range(-0.02..0.02),
                                        ..Default::default()
                                    },
                                    GeometryBuilder::build_as(
                                        &shape,
                                        DrawMode::Fill(FillMode::color(
                                            Color::hex("444444").unwrap(),
                                        )),
                                        asteroid_transform,
                                    ),
                                ))
                                .id();

                            if !is_destructible {
                                commands.entity(entity).insert(Particle::new(
                                    Vec2::new(0., 0.),
                                    JETPACK_PARTICLE_LIFETIME, // FIXME
                                ));
                            }
                        }

                        commands.entity(entity).despawn();
                    }
                }
                println!("Entity {:?} hit at point {}", entity, hit_point);
            }

            let offset = match laser.position {
                Direction::LEFT => -0.22,
                Direction::RIGHT => 0.28,
            };

            let line = shapes::Line(Vec2::ZERO, direction * laser.height);
            *path = ShapePath::build_as(&line);
            transform.translation = Vec3::new(
                center.x + rotation_as_vector.x.cos() * offset + direction.x * 5.,
                center.y + rotation_as_vector.y.sin() * offset + direction.y * 5.,
                0.2,
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
    }
}

fn polygon_area(vertices: &Vec<Vec2>) -> f32 {
    let mut sum = 0.0;
    for i in 0..vertices.len() {
        let v1 = vertices[i];
        let v2 = vertices[(i + 1) % vertices.len()];
        sum += v1.x * v2.y - v1.y * v2.x;
    }
    sum.abs() / 2.0
}

fn subdivide(collider: &Collider) -> Vec<Collider> {
    let vertices: Vec<Vec2> = collider.as_convex_polygon().unwrap().points().collect();
    let centroid = compute_polygon_centroid(&vertices);
    let triangles = create_triangles_using_centroid(centroid, &vertices);
    let colliders: Vec<Collider> = triangles
        .iter()
        .as_slice()
        .chunks(2)
        .filter_map(|chunk| {
            let mut triangle1 = chunk[0].clone();
            if let Some(triangle2) = chunk.get(1) {
                let mut triangle2 = triangle2.clone();
                triangle1.append(&mut triangle2);
            }
            Collider::convex_hull(&triangle1)
        })
        .filter(|collider| {
            let new_vertices: Vec<Vec2> = collider.as_convex_polygon().unwrap().points().collect();
            vertices != new_vertices
        })
        .collect();
    colliders
}

fn compute_polygon_centroid(vertices: &[Vec2]) -> Vec2 {
    let n = vertices.len();
    let mut sum = Vec2::new(0.0, 0.0);
    let mut area = 0.0;

    for i in 0..n {
        let j = (i + 1) % n;
        let a = vertices[i];
        let b = vertices[j];
        let cross = a.x * b.y - b.x * a.y;
        sum += (a + b) * cross;
        area += cross;
    }

    sum / (3.0 * area)
}

fn create_triangles_using_centroid(centroid: Vec2, vertices: &[Vec2]) -> Vec<Vec<Vec2>> {
    let n = vertices.len();
    let mut triangles = Vec::with_capacity(n);

    for i in 0..n {
        let j = (i + 1) % n;

        triangles.push(vec![centroid, vertices[i], vertices[j]]);
    }

    triangles
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
        impulse.impulse = direction * 1.5;
    }
}

fn player_spawn(mut commands: Commands, asset_server: Res<AssetServer>) {
    let half_body_width = 110. * PIXEL_TO_METERS;
    let half_body_height = 280. * PIXEL_TO_METERS;

    let collider = Collider::cuboid(half_body_width, half_body_height);

    let transform = Transform::from_xyz(0.0, half_body_height, 0.);

    let mut prev_id = commands
        .spawn((
            Player,
            OnGameScreen,
            RigidBody::Dynamic,
            collider,
            Velocity::default(),
            ExternalImpulse::default(),
            Damping { linear_damping: 0.5, angular_damping: 1.0 },
            SpriteBundle {
                texture: asset_server.load("dino.png"),
                sprite: Sprite {
                    custom_size: Some(Vec2::new(half_body_width * 2., half_body_height * 2.)),
                    ..Default::default()
                },
                transform,
                ..Default::default()
            },
            IsShooting(false),
            Controls::default(),
        ))
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
            .spawn((
                RigidBody::Dynamic,
                OnGameScreen,
                collider,
                GeometryBuilder::build_as(
                    &shape,
                    DrawMode::Fill(FillMode::color(Color::hex("26b24a").unwrap())),
                    Transform::from_xyz(0., pos_y, 0.),
                ),
                ImpulseJoint::new(prev_id, joint),
                ColliderMassProperties::Density(100.0),
            ))
            .id();

        pos_y -= half_height * 2.;
        half_width -= 0.05;
        prev_id = id;
        prev_half_height = half_height;
    }
}

fn asteroids_spawn(mut commands: Commands) {
    let mut i = 0;
    let total = 20;
    let margin = 5.;
    let mut translations: Vec<Vec2> = Vec::with_capacity(total);
    let max_radius = 6.;

    while i < 30 {
        let mut rng = thread_rng();
        let area_radius = 50.;
        let asteroid_radius = rng.gen_range(2.0..max_radius);
        let translation = Vec2::new(
            rng.gen_range(-area_radius..area_radius),
            rng.gen_range(-area_radius..area_radius),
        );

        let too_close_to_others_asteroid = translations.iter().any(|&other_translation| {
            (other_translation - translation).length() <= max_radius + margin
        });

        let too_close_to_player = translation.x + max_radius > -110. * PIXEL_TO_METERS
            && translation.x - max_radius < 110. * PIXEL_TO_METERS
            && translation.y + max_radius > -280. * PIXEL_TO_METERS
            && translation.y - max_radius < 280. * PIXEL_TO_METERS;

        if too_close_to_others_asteroid || too_close_to_player {
            continue;
        }
        i = i + 1;

        translations.push(translation.clone());

        let num_points = 50;
        let mut vertices = Vec::with_capacity(num_points);
        for _ in 0..num_points {
            let angle = rng.gen_range(0.0..std::f32::consts::TAU);
            let radius = rng.gen_range(0.0..asteroid_radius);
            let x = radius * angle.cos();
            let y = radius * angle.sin();
            vertices.push(Vec2::new(x, y));
        }

        let asteroid_collider = Collider::convex_hull(&vertices).unwrap();
        let points: Vec<Vec2> = asteroid_collider
            .as_convex_polygon()
            .unwrap()
            .points()
            .collect();
        let shape = shapes::Polygon::from(Polygon {
            points: points,
            closed: true,
        });
        commands.spawn((
            Asteroid,
            OnGameScreen,
            RigidBody::Dynamic,
            Destructible(true),
            asteroid_collider,
            ColliderMassProperties::Density(10.0),
            ExternalForce {
                torque: rng.gen_range(-0.01..0.01),
                force: Vec2::new(rng.gen_range(-1.5..1.5), rng.gen_range(-1.5..1.5)),
            },
            GeometryBuilder::build_as(
                &shape,
                DrawMode::Fill(FillMode::color(Color::hex("444444").unwrap())),
                Transform::from_translation(Vec3::new(translation.x, translation.y, 0.)),
            ),
        ));
    }
}

fn monsters_spawn(mut commands: Commands, asset_server: Res<AssetServer>) {
    let half_body_width = 198. * PIXEL_TO_METERS;
    let half_body_height = 184. * PIXEL_TO_METERS;

    let collider = Collider::cuboid(half_body_width, half_body_height);
    commands.spawn((
        Monster,
        OnGameScreen,
        RigidBody::Dynamic,
        collider,
        ExternalForce::default(),
        Damping { linear_damping: 0.5, angular_damping: 1.0 },
        SeekBehavior {
            target: Vec2::ZERO,
            position: Vec2::ZERO,
            max_speed: 10.,
        },
        KeepVerticalBehavior {
            angle: 0.,
            max_speed: 0.8,
        },
        SpriteBundle {
            texture: asset_server.load("monster.png"),
            sprite: Sprite {
                custom_size: Some(Vec2::new(half_body_width * 2., half_body_height * 2.)),
                ..Default::default()
            },
            transform: Transform::from_translation(Vec3::new(-5., -5., 0.)),
            ..Default::default()
        },
    ));
}

fn camera(
    mut camera_transform_query: Query<&mut Transform, (With<super::MainCamera>, Without<Player>)>,
    player_transform_query: Query<&Transform, With<Player>>,
) {
    let mut camera_transform = camera_transform_query.single_mut();
    let player_transform = player_transform_query.single();
    camera_transform.translation = player_transform.translation;
    camera_transform.translation.z = 1.;
}
