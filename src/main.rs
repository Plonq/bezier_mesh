use bevy::math::Vec3Swizzles;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy_inspector_egui::prelude::*;
use bevy_inspector_egui::quick::{ResourceInspectorPlugin, WorldInspectorPlugin};
use bevy_mod_picking::{DefaultPickingPlugins, PickableBundle, PickingCameraBundle};
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use bevy_transform_gizmo::{GizmoPickSource, GizmoTransformable, TransformGizmoPlugin};
use bevy_vector_shapes::prelude::*;
use itertools::Itertools;
use std::f32::consts::{PI, TAU};

fn main() {
    App::new()
        .insert_resource(Config { detail: 20 })
        .register_type::<Config>()
        .add_plugins(DefaultPlugins)
        .add_plugin(PanOrbitCameraPlugin)
        .add_plugin(ShapePlugin {
            base_config: ShapeConfig {
                alignment: Alignment::Billboard,
                ..default()
            },
        })
        .add_plugin(ResourceInspectorPlugin::<Config>::default())
        .add_plugin(WorldInspectorPlugin::default())
        .add_plugins(DefaultPickingPlugins)
        .add_plugin(TransformGizmoPlugin::default())
        .add_startup_systems((setup, build_mesh).chain())
        // .add_system(debug)
        .run()
}

#[derive(Component, Default, Debug)]
struct ControlPoint(usize);

#[derive(Component, Default, Debug, Reflect)]
#[reflect(Component)]
struct Velocity(Vec2);

#[derive(Reflect, Resource, Default, InspectorOptions)]
#[reflect(Resource, InspectorOptions)]
struct Config {
    detail: usize,
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    config: Res<Config>,
) {
    // Ground
    // commands.spawn(PbrBundle {
    //     mesh: meshes.add(shape::Plane::from_size(5.0).into()),
    //     material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
    //     ..default()
    // });
    // Cube
    // commands.spawn(PbrBundle {
    //     mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
    //     material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
    //     transform: Transform::from_xyz(0.0, 0.5, 0.0),
    //     ..default()
    // });
    // Light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });
    // Camera
    commands.spawn((
        Camera3dBundle::default(),
        PanOrbitCamera {
            radius: 10.0,
            button_orbit: MouseButton::Middle,
            modifier_pan: Some(KeyCode::LShift),
            button_pan: MouseButton::Middle,
            ..default()
        },
        PickingCameraBundle::default(),
        GizmoPickSource::default(),
    ));

    let control_points = (0..4)
        .map(|i| Vec3::new(i as f32 * 3.0, 0.0, 0.0))
        .collect::<Vec<_>>();

    // Control point meshes
    for (i, point) in control_points.iter().enumerate() {
        commands.spawn((
            ControlPoint(i),
            PbrBundle {
                mesh: meshes.add(Mesh::from(shape::UVSphere {
                    radius: 0.05,
                    ..default()
                })),
                material: materials.add(Color::RED.into()),
                transform: Transform::from_translation(*point),
                ..default()
            },
            PickableBundle::default(),
            GizmoTransformable,
        ));
    }

    let p1 = control_points[0];
    let p2 = control_points[1];
    let p3 = control_points[2];
    let p4 = control_points[3];
    let vertices = (0..=config.detail)
        .map(|i| i as f32 / config.detail as f32)
        .map(|t| (t, cubic_bezier(p1, p2, p3, p4, t)))
        .flat_map(|(t, curve_point)| {
            // Vertices of one slice of road, relative to the point on the curve
            let local_vertices = vec![
                Vec3::new(-0.5, 0.3, 0.0),
                Vec3::new(-0.3, 0.3, 0.0),
                Vec3::new(-0.2, 0.2, 0.0),
                Vec3::new(0.2, 0.2, 0.0),
                Vec3::new(0.3, 0.3, 0.0),
                Vec3::new(0.5, 0.3, 0.0),
                Vec3::new(0.5, 0.0, 0.0),
                Vec3::new(-0.5, 0.0, 0.0),
            ];
            // Map these local points to world points by adding them to the curve point
            local_vertices.into_iter().map(move |local_vertex| {
                let prev = cubic_bezier(p1, p2, p3, p4, t - 0.01);
                let next = cubic_bezier(p1, p2, p3, p4, t + 0.01);
                // Create little local space basis vectors, pointing along the curve
                let local_z = (prev - next).normalize();
                let local_y = Vec3::Y;
                let local_x = local_y.cross(local_z).normalize();
                // Convert local coordinates to world coordinates
                let mut world_vertex = curve_point;
                world_vertex += local_vertex.x * local_x;
                world_vertex += local_vertex.y * local_y;
                world_vertex += local_vertex.z * local_z;

                world_vertex
            })
        })
        .collect::<Vec<_>>();

    // debug
    for v in vertices.iter() {
        println!("spawning at: {v}");
        commands.spawn((PbrBundle {
            mesh: meshes.add(Mesh::from(shape::UVSphere {
                radius: 0.02,
                ..default()
            })),
            material: materials.add(Color::RED.into()),
            transform: Transform::from_translation(*v),
            ..default()
        },));
    }

    let triangles: Vec<u32> = vec![0, 8, 15, 0, 15, 7, 0, 9, 8, 0, 1, 9];

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    mesh.set_indices(Some(Indices::U32(triangles)));
    mesh.duplicate_vertices();
    mesh.compute_flat_normals();
    let handle = meshes.add(mesh);

    commands.spawn(PbrBundle {
        mesh: handle,
        material: materials.add(StandardMaterial {
            base_color: Color::ORANGE_RED,
            ..default()
        }),
        ..default()
    });
}

fn build_mesh(
    config: Res<Config>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    point_q: Query<(&ControlPoint, &Transform), With<ControlPoint>>,
) {
    if let Some((tfm1, tfm2, tfm3, tfm4)) = point_q
        .iter()
        .sorted_by_key(|(p, _)| p.0)
        .map(|(_, tfm)| tfm)
        .tuples::<(_, _, _, _)>()
        .last()
    {
        println!("Has transforms");
        let vertices = (0..=config.detail)
            .map(|i| i as f32 / config.detail as f32)
            .map(|t| {
                (
                    t,
                    cubic_bezier(
                        tfm1.translation,
                        tfm2.translation,
                        tfm3.translation,
                        tfm4.translation,
                        t,
                    ),
                )
            })
            .flat_map(|(t, curve_point)| {
                // Vertices of one slice of road, relative to the point on the curve
                let local_vertices = vec![
                    Vec3::new(-0.5, 0.3, 0.0),
                    Vec3::new(-0.3, 0.3, 0.0),
                    Vec3::new(-0.2, 0.2, 0.0),
                    Vec3::new(0.2, 0.2, 0.0),
                    Vec3::new(0.3, 0.3, 0.0),
                    Vec3::new(0.5, 0.3, 0.0),
                    Vec3::new(0.5, 0.0, 0.0),
                    Vec3::new(-0.5, 0.0, 0.0),
                ];
                // Map these local points to world points by adding them to the curve point
                local_vertices.into_iter().map(move |local_vertex| {
                    let prev = cubic_bezier(
                        tfm1.translation,
                        tfm2.translation,
                        tfm3.translation,
                        tfm4.translation,
                        t - 0.01,
                    );
                    let next = cubic_bezier(
                        tfm1.translation,
                        tfm2.translation,
                        tfm3.translation,
                        tfm4.translation,
                        t + 0.01,
                    );
                    // Create little local space basis vectors, pointing along the curve
                    let local_z = (prev - next).normalize();
                    let local_y = Vec3::Y;
                    let local_x = local_y.cross(local_z).normalize();
                    // Convert local coordinates to world coordinates
                    let mut world_vertex = curve_point;
                    world_vertex += local_vertex.x * local_x;
                    world_vertex += local_vertex.y * local_y;
                    world_vertex += local_vertex.z * local_z;

                    world_vertex
                })
            })
            .collect::<Vec<_>>();

        // debug
        for v in vertices.iter() {
            println!("spawning at: {v}");
            commands.spawn((PbrBundle {
                mesh: meshes.add(Mesh::from(shape::UVSphere {
                    radius: 0.02,
                    ..default()
                })),
                material: materials.add(Color::RED.into()),
                transform: Transform::from_translation(*v),
                ..default()
            },));
        }

        let triangles: Vec<u32> = vec![0, 8, 15, 0, 15, 7, 0, 9, 8, 0, 1, 9];

        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
        mesh.set_indices(Some(Indices::U32(triangles)));
        mesh.duplicate_vertices();
        mesh.compute_flat_normals();
        let handle = meshes.add(mesh);

        commands.spawn(PbrBundle {
            mesh: handle,
            material: materials.add(StandardMaterial {
                base_color: Color::ORANGE_RED,
                ..default()
            }),
            ..default()
        });
    }
}

fn debug(
    config: Res<Config>,
    mut painter: ShapePainter,
    point_q: Query<(&ControlPoint, &Transform), With<ControlPoint>>,
) {
    painter.color = Color::RED;
    painter.thickness = 0.01;
    painter.cap = Cap::Round;
    painter.set_translation(Vec3::ZERO);

    for ((_, tform1), (_, tform2), (_, tform3), (_, tform4)) in point_q
        .iter()
        .sorted_by_key(|(p, _)| p.0)
        .tuples::<(_, _, _, _)>()
    {
        let points = (0..=config.detail)
            .map(|i| i as f32 / config.detail as f32)
            .map(|t| {
                cubic_bezier(
                    tform1.translation,
                    tform2.translation,
                    tform3.translation,
                    tform4.translation,
                    t,
                )
            })
            .map(|v| (v, Color::RED))
            .collect::<Vec<_>>();

        draw_polyline(points, &mut painter);
    }

    // if let Some((tfm1, tfm2, tfm3, tfm4)) = point_q
    //     .iter()
    //     .sorted_by_key(|(p, _)| p.0)
    //     .map(|(_, tfm)| tfm)
    //     .tuples::<(_, _, _, _)>()
    //     .last()
    // {
    //     let vertices = (0..=config.detail)
    //         .map(|i| i as f32 / config.detail as f32)
    //         .map(|t| {
    //             (
    //                 t,
    //                 cubic_bezier(
    //                     tfm1.translation,
    //                     tfm2.translation,
    //                     tfm3.translation,
    //                     tfm4.translation,
    //                     t,
    //                 ),
    //             )
    //         })
    //         .flat_map(|(t, curve_point)| {
    //             // Vertices of one slice of road, relative to the point on the curve
    //             let local_vertices = vec![
    //                 Vec3::new(-0.5, 0.3, 0.0),
    //                 Vec3::new(-0.3, 0.3, 0.0),
    //                 Vec3::new(-0.2, 0.2, 0.0),
    //                 Vec3::new(0.2, 0.2, 0.0),
    //                 Vec3::new(0.3, 0.3, 0.0),
    //                 Vec3::new(0.5, 0.3, 0.0),
    //                 Vec3::new(0.5, 0.0, 0.0),
    //                 Vec3::new(-0.5, 0.0, 0.0),
    //             ];
    //             // Map these local points to world points by adding them to the curve point
    //             local_vertices.into_iter().map(move |local_vertex| {
    //                 let prev = cubic_bezier(
    //                     tfm1.translation,
    //                     tfm2.translation,
    //                     tfm3.translation,
    //                     tfm4.translation,
    //                     t - 0.01,
    //                 );
    //                 let next = cubic_bezier(
    //                     tfm1.translation,
    //                     tfm2.translation,
    //                     tfm3.translation,
    //                     tfm4.translation,
    //                     t + 0.01,
    //                 );
    //                 let local_z = (prev - next).normalize();
    //                 let local_y = Vec3::Y;
    //                 let local_x = local_y.cross(local_z).normalize();
    //                 let mut world_vertex = curve_point;
    //                 world_vertex += local_vertex.x * local_x;
    //                 world_vertex += local_vertex.y * local_y;
    //                 world_vertex += local_vertex.z * local_z;
    //                 world_vertex
    //             })
    //         })
    //         .collect::<Vec<_>>();
    //
    //     painter.hollow = false;
    //     painter.color = Color::ORANGE;
    //     for v in vertices {
    //         painter.set_translation(v);
    //         painter.circle(0.015);
    //     }
    // }
}

fn cubic_bezier(a: Vec3, b: Vec3, c: Vec3, d: Vec3, t: f32) -> Vec3 {
    let ab = a.lerp(b, t);
    let bc = b.lerp(c, t);
    let cd = c.lerp(d, t);
    let abbc = ab.lerp(bc, t);
    let bccd = bc.lerp(cd, t);
    abbc.lerp(bccd, t)
}

fn draw_polyline(points: Vec<(Vec3, Color)>, painter: &mut ShapePainter) {
    for window in points.windows(2) {
        let (point_1, color_1) = window[0];
        let (point_2, _) = window[1];
        painter.color = color_1;
        painter.line(point_1, point_2);
    }
}