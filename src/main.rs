use crate::material::UvDebugMaterial;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy_inspector_egui::prelude::*;
use bevy_inspector_egui::quick::{ResourceInspectorPlugin, WorldInspectorPlugin};
use bevy_mod_picking::{DefaultPickingPlugins, PickableBundle, PickingCameraBundle};
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use bevy_transform_gizmo::{GizmoPickSource, GizmoTransformable, TransformGizmoPlugin};
use bevy_vector_shapes::prelude::*;
use itertools::Itertools;
use std::f32::consts::{FRAC_1_SQRT_2, TAU};

mod material;

fn main() {
    App::new()
        .insert_resource(Config {
            detail: 20,
            control_points: (0..4)
                .map(|i| Vec3::new(i as f32 * 3.0, 0.0, 0.0))
                .collect(),
            auto_update: true,
            ..default()
        })
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
        .add_plugin(MaterialPlugin::<UvDebugMaterial>::default())
        .add_startup_system(setup)
        .add_systems((build_mesh.run_if(|config: Res<Config>| config.auto_update),).chain())
        .run()
}

#[derive(Component, Default, Debug)]
struct ControlPoint(usize);

#[derive(Component, Default, Debug)]
struct Generated;

#[derive(Component, Default, Debug, Reflect)]
#[reflect(Component)]
struct Velocity(Vec2);

#[derive(Reflect, Resource, Default, InspectorOptions)]
#[reflect(Resource, InspectorOptions)]
struct Config {
    auto_update: bool,
    #[inspector(min = 2, max = 150)]
    detail: usize,
    control_points: Vec<Vec3>,
    mesh: Option<Handle<Mesh>>,
}

#[derive(Default)]
struct Vertex {
    point: Vec3,
    normal: Vec3,
    uv: Vec2,
}

impl Vertex {
    fn new(point: Vec3, normal: Vec3, uv: Vec2) -> Self {
        Vertex { point, normal, uv }
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    config: Res<Config>,
) {
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

    // Control point meshes
    for (i, point) in config.control_points.iter().enumerate() {
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
}

fn build_mesh(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut debug_materials: ResMut<Assets<UvDebugMaterial>>,
    point_q: Query<(&ControlPoint, &Transform)>,
    mut config: ResMut<Config>,
    asset_server: Res<AssetServer>,
    mut painter: ShapePainter,
) {
    if let Some(((_, tfm1), (_, tfm2), (_, tfm3), (_, tfm4))) = point_q
        .iter()
        .sorted_by_key(|(cp, _)| cp.0)
        .tuples::<(_, _, _, _)>()
        .last()
    {
        let vertices = (0..config.detail)
            .map(|i| i as f32 / (config.detail as f32 - 1.0))
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
                #[rustfmt::skip]
                let local_vertices = vec![
                    // 0
                    Vertex::new(Vec3::new(-0.5, 0.3, 0.0), Vec3::NEG_X, Vec2::new(0.0, t)),
                    Vertex::new(Vec3::new(-0.5, 0.3, 0.0), Vec3::Y, Vec2::new(0.0, t)),
                    // 1
                    Vertex::new(Vec3::new(-0.3, 0.3, 0.0), Vec3::Y, Vec2::new(0.05, t)),
                    Vertex::new(Vec3::new(-0.3, 0.3, 0.0), Vec3::new(FRAC_1_SQRT_2, FRAC_1_SQRT_2, 0.0), Vec2::new(0.05, t)),
                    // 2
                    Vertex::new(Vec3::new(-0.2, 0.2, 0.0), Vec3::new(FRAC_1_SQRT_2, FRAC_1_SQRT_2, 0.0), Vec2::new(0.1, t)),
                    Vertex::new(Vec3::new(-0.2, 0.2, 0.0), Vec3::Y, Vec2::new(0.1, t)),
                    // 3
                    Vertex::new(Vec3::new(0.2, 0.2, 0.0), Vec3::Y, Vec2::new(0.9, t)),
                    Vertex::new(Vec3::new(0.2, 0.2, 0.0), Vec3::new(-FRAC_1_SQRT_2, FRAC_1_SQRT_2, 0.0), Vec2::new(0.9, t)),
                    // 4
                    Vertex::new(Vec3::new(0.3, 0.3, 0.0), Vec3::new(-FRAC_1_SQRT_2, FRAC_1_SQRT_2, 0.0), Vec2::new(0.95, t)),
                    Vertex::new(Vec3::new(0.3, 0.3, 0.0), Vec3::Y, Vec2::new(0.95, t)),
                    // 5
                    Vertex::new(Vec3::new(0.5, 0.3, 0.0), Vec3::Y, Vec2::new(1.0, t)),
                    Vertex::new(Vec3::new(0.5, 0.3, 0.0), Vec3::X, Vec2::new(1.0, t)),
                    // 6
                    Vertex::new(Vec3::new(0.5, 0.0, 0.0), Vec3::X, Vec2::new(1.0, t)),
                    Vertex::new(Vec3::new(0.5, 0.0, 0.0), Vec3::NEG_Y, Vec2::new(1.0, t)),
                    // 7
                    Vertex::new(Vec3::new(-0.5, 0.0, 0.0), Vec3::NEG_Y, Vec2::new(1.0, t)),
                    Vertex::new(Vec3::new(-0.5, 0.0, 0.0), Vec3::NEG_X, Vec2::new(1.0, t)),
                ];

                // Map these local points to world points by adding them to the curve point
                local_vertices.into_iter().map(move |mut local_vertex| {
                    let bez_mat = cubic_bezier_matrix(
                            tfm1.translation,
                            tfm2.translation,
                            tfm3.translation,
                            tfm4.translation,
                            t,
                    );
                    local_vertex.point = bez_mat.transform_point3(local_vertex.point);
                    local_vertex.normal = bez_mat.transform_vector3(local_vertex.normal);
                    local_vertex
                })
            })
            .collect::<Vec<_>>();

        // debug
        // for v in vertices.iter() {
        //     // println!("spawning at: {v}");
        //     commands.spawn((
        //         PbrBundle {
        //             mesh: meshes.add(Mesh::from(shape::UVSphere {
        //                 radius: 0.02,
        //                 ..default()
        //             })),
        //             material: materials.add(Color::RED.into()),
        //             transform: Transform::from_translation(*v),
        //             ..default()
        //         },
        //         Generated,
        //     ));
        // }

        // Debug normals
        // painter.thickness = 0.005;
        // painter.cap = Cap::None;
        // for v in vertices.iter() {
        //     let color = Color::rgb(v.normal.x, v.normal.y, v.normal.z);
        //     let dest = v.point + v.normal * 0.15;
        //     painter.color = color;
        //     painter.line(v.point, dest);
        // }

        let mut triangles: Vec<u32> = vec![];
        for i in 0..(config.detail - 1) {
            #[rustfmt::skip]
                let base_tris: Vec<u32> = vec![
                0, 16,31,
                1, 18,17,
                1, 2, 18,
                3, 20,19,
                3, 4, 20,
                5, 22,21,
                5, 6, 22,
                7, 24,23,
                7, 8, 24,
                9, 26,25,
                9, 10,26,
                11,28,27,
                11,12,28,
                13,30,29,
                13,14,30,
                15,16,31,
                15,0, 16,
            ];
            for j in base_tris {
                triangles.push(j + (i * 16) as u32);
            }
        }

        let vert_points = vertices.iter().map(|v| v.point).collect::<Vec<_>>();
        let vert_normals = vertices.iter().map(|v| v.normal).collect::<Vec<_>>();
        let vert_uvs = vertices.iter().map(|v| v.uv).collect::<Vec<_>>();

        if let Some(mesh_handle) = &config.mesh {
            let mesh = meshes.get_mut(mesh_handle).unwrap();
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vert_points);
            mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vert_normals);
            mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, vert_uvs);
            mesh.set_indices(Some(Indices::U32(triangles)));
        } else {
            let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vert_points);
            mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vert_normals);
            mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, vert_uvs);
            mesh.set_indices(Some(Indices::U32(triangles)));
            let handle = meshes.add(mesh);

            let road_tex_handle = asset_server.load("road.png");

            commands.spawn((
                Generated,
                PbrBundle {
                    mesh: handle.clone(),
                    material: materials.add(StandardMaterial {
                        base_color_texture: Some(road_tex_handle),
                        ..default()
                    }),
                    ..default()
                },
                // MaterialMeshBundle {
                //     mesh: handle.clone(),
                //     material: debug_materials.add(UvDebugMaterial::default()),
                //     ..default()
                // },
            ));

            config.mesh = Some(handle);
        }
    }
}

fn cubic_bezier(a: Vec3, b: Vec3, c: Vec3, d: Vec3, t: f32) -> Vec3 {
    let ab = a.lerp(b, t);
    let bc = b.lerp(c, t);
    let cd = c.lerp(d, t);
    let abbc = ab.lerp(bc, t);
    let bccd = bc.lerp(cd, t);
    abbc.lerp(bccd, t)
}

fn cubic_bezier_matrix(a: Vec3, b: Vec3, c: Vec3, d: Vec3, t: f32) -> Mat4 {
    let ab = a.lerp(b, t);
    let bc = b.lerp(c, t);
    let cd = c.lerp(d, t);
    let abbc = ab.lerp(bc, t);
    let bccd = bc.lerp(cd, t);
    let position = abbc.lerp(bccd, t);
    let z = (abbc - bccd).normalize();
    let y = Vec3::Y;
    let x = y.cross(z);
    Mat4::from_cols(
        Vec4::from((x, 0.0)),
        Vec4::from((y, 0.0)),
        Vec4::from((z, 0.0)),
        Vec4::from((position, 1.0)),
    )
}

fn draw_polyline(points: Vec<(Vec3, Color)>, painter: &mut ShapePainter) {
    for window in points.windows(2) {
        let (point_1, color_1) = window[0];
        let (point_2, _) = window[1];
        painter.color = color_1;
        painter.line(point_1, point_2);
    }
}
