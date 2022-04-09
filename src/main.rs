use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugin(RapierRenderPlugin)
        .insert_resource(RapierConfiguration {
            gravity: Vector::new(0., 0.),
            ..Default::default()
        })
        .add_startup_system(setup)
        .add_system(movement)
        .add_system(player)
        .run();
}

#[derive(Component)]
struct Player;

enum Rotation {
    LEFT,
    RIGHT
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

fn player(mut query: Query<(&Movement, &RigidBodyPositionComponent, &RigidBodyMassPropsComponent, &mut RigidBodyVelocityComponent), With<Player>>) {
	let (movement, rb_pos, rb_mprops, mut rb_vel) = query.single_mut();
    rb_vel.angvel = match movement.rotation {
        | Some(Rotation::LEFT) => 5.,
        | Some(Rotation::RIGHT) => -5.,
        | None => 0.0,
    };
    rb_vel.linvel *= 0.99;
    let angle = rb_pos.position.rotation.angle();
    if movement.propulsion {
        let impulse = Vec2::new(-angle.sin() * 20., angle.cos() * 20.);
        rb_vel.apply_impulse(rb_mprops, impulse.into());
    }

}

fn setup(mut commands: Commands) {
    let mut camera_bundle = OrthographicCameraBundle::new_2d();
    camera_bundle.orthographic_projection.scale = 0.1;
    commands.spawn_bundle(camera_bundle);

    let body_width = 2.;
    let body_height = 4.;

    let collider = ColliderBundle {
        shape: ColliderShape::cuboid(body_width, body_height).into(),
        ..Default::default()
    };

    let rigid_body = RigidBodyBundle {
        position: Vec2::new(0.0, 20. + body_height).into(),
        ..Default::default()
    };

    let mut prev_id = commands
        .spawn_bundle(rigid_body)
        .insert_bundle(collider)
        .insert(Player)
        .insert(Movement::default())
        .insert(ColliderPositionSync::Discrete)
        .insert(ColliderDebugRender::from(Color::BLUE))
        .id();

    let mut pos = Vec2::new(0.0, 20.0);
    let mut prev_half_height = body_height;
    let mut half_width = 0.6;

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

        let id = commands
            .spawn_bundle(rigid_body)
            .insert_bundle(collider)
            .insert(ColliderPositionSync::Discrete)
            .insert(ColliderDebugRender::from(Color::BLUE))
            .id();

        commands.spawn_bundle((
            JointBuilderComponent::new(
                RevoluteJoint::new()
                    .local_anchor1(point![0.0, -prev_half_height])
                    .local_anchor2(point![0.0, half_height])
                    .limit_axis([-10.,10.]),
                prev_id,
                id,
            ),
        ));

        pos -= Vec2::new(0., half_height*2.);
        half_width -= 0.05;
        prev_id = id;
        prev_half_height = half_height;
    }

}