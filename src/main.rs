use amethyst::renderer::light::{Light, PointLight};
use amethyst::renderer::palette::rgb::Rgb;
use amethyst::{
    animation::{
        get_animation_set, AnimationBundle, AnimationCommand, AnimationControlSet, AnimationSet,
        EndControl, VertexSkinningBundle,
    },
    assets::{
        AssetLoaderSystemData, AssetPrefab, Completion, Handle, Prefab, PrefabData, PrefabLoader,
        PrefabLoaderSystemDesc, ProgressCounter, RonFormat,
    },
    controls::{ControlTagPrefab, FlyControlBundle},
    core::transform::{Transform, TransformBundle},
    derive::PrefabData,
    ecs::{Entity, ReadStorage, Write, WriteStorage},
    input::{is_close_requested, is_key_down, StringBindings, VirtualKeyCode},
    prelude::*,
    renderer::{
        camera::Camera,
        camera::CameraPrefab,
        light::LightPrefab,
        mtl::{Material, MaterialDefaults},
        plugins::{RenderPbr3D, RenderSkybox, RenderToWindow},
        rendy::mesh::{Normal, Position, Tangent, TexCoord},
        shape::Shape,
        types::DefaultBackend,
        Mesh, RenderingBundle,
    },
    utils::{
        application_root_dir,
        auto_fov::{AutoFov, AutoFovSystem},
        tag::{Tag, TagFinder},
    },
    Error,
};
use amethyst_gltf::{GltfSceneAsset, GltfSceneFormat, GltfSceneLoaderSystemDesc};

use serde::{Deserialize, Serialize};

#[derive(Default)]
struct GameState {
    entity: Option<Entity>,
    initialised: bool,
    progress: Option<ProgressCounter>,
}

#[derive(Clone, Serialize, Deserialize)]
struct AnimationMarker;

#[derive(Default)]
struct Scene {
    handle: Option<Handle<Prefab<ScenePrefabData>>>,
    animation_index: usize,
}

#[derive(Default, Deserialize, Serialize, PrefabData)]
#[serde(default)]
struct ScenePrefabData {
    transform: Option<Transform>,
    gltf: Option<AssetPrefab<GltfSceneAsset, GltfSceneFormat>>,
    camera: Option<CameraPrefab>,
    auto_fov: Option<AutoFov>,
    light: Option<LightPrefab>,
    tag: Option<Tag<AnimationMarker>>,
    fly_tag: Option<ControlTagPrefab>,
}

impl SimpleState for GameState {
    fn on_start(&mut self, data: StateData<'_, GameData<'_, '_>>) {
        let StateData { world, .. } = data;
        self.progress = Some(ProgressCounter::default());

        world.exec(
            |(loader, mut scene): (PrefabLoader<'_, ScenePrefabData>, Write<'_, Scene>)| {
                scene.handle = Some(loader.load(
                    "prefab/mclaren_mp4-12c.ron",
                    // "prefab/renderable.ron",
                    RonFormat,
                    self.progress.as_mut().unwrap(),
                ));
            },
        );

        // initialize_camera(state_data.world);
        // initialize_player(state_data.world);
        // initialize_light(state_data.world);
    }

    fn handle_event(
        &mut self,
        data: StateData<'_, GameData<'_, '_>>,
        event: StateEvent,
    ) -> SimpleTrans {
        let StateData { world, .. } = data;
        if let StateEvent::Window(event) = &event {
            if is_close_requested(&event) || is_key_down(&event, VirtualKeyCode::Escape) {
                Trans::Quit
            } else if is_key_down(&event, VirtualKeyCode::Space) {
                // toggle_or_cycle_animation(
                //     self.entity,
                //     &mut world.write_resource(),
                //     &world.read_storage(),
                //     &mut world.write_storage(),
                // );
                Trans::None
            } else {
                Trans::None
            }
        } else {
            Trans::None
        }
    }

    fn update(&mut self, data: &mut StateData<'_, GameData<'_, '_>>) -> SimpleTrans {
        if !self.initialised {
            let remove = match self.progress.as_ref().map(|p| p.complete()) {
                None | Some(Completion::Loading) => false,

                Some(Completion::Complete) => {
                    let scene_handle = data
                        .world
                        .read_resource::<Scene>()
                        .handle
                        .as_ref()
                        .unwrap()
                        .clone();

                    data.world.create_entity().with(scene_handle).build();

                    true
                }

                Some(Completion::Failed) => {
                    println!("Error: {:?}", self.progress.as_ref().unwrap().errors());
                    return Trans::Quit;
                }
            };
            if remove {
                self.progress = None;
            }
            if self.entity.is_none() {
                if let Some(entity) = data
                    .world
                    .exec(|finder: TagFinder<'_, AnimationMarker>| finder.find())
                {
                    self.entity = Some(entity);
                    self.initialised = true;
                }
            }
        }
        Trans::None
    }
}

fn main() -> Result<(), amethyst::Error> {
    amethyst::start_logger(Default::default());

    let app_root = application_root_dir()?;

    let config_dir = app_root.join("config/");
    let display_config_path = config_dir.join("display.ron");
    let assets_dir = app_root.join("assets/");

    let game_data = GameDataBuilder::default()
        .with(AutoFovSystem::default(), "auto_fov", &[])
        .with_system_desc(
            PrefabLoaderSystemDesc::<ScenePrefabData>::default(),
            "scene_loader",
            &[],
        )
        .with_system_desc(
            GltfSceneLoaderSystemDesc::default(),
            "gltf_loader",
            &["scene_loader"], // This is important so that entity instantiation is performed in a single frame.
        )
        .with_bundle(
            AnimationBundle::<usize, Transform>::new("animation_control", "sampler_interpolation")
                .with_dep(&["gltf_loader"]),
        )?
        .with_bundle(
            FlyControlBundle::<StringBindings>::new(None, None, None)
                .with_sensitivity(0.1, 0.1)
                .with_speed(5.),
        )?
        .with_bundle(TransformBundle::new().with_dep(&[
            "animation_control",
            "sampler_interpolation",
            "fly_movement",
        ]))?
        .with_bundle(VertexSkinningBundle::new().with_dep(&[
            "transform_system",
            "animation_control",
            "sampler_interpolation",
        ]))?
        .with_bundle(
            RenderingBundle::<DefaultBackend>::new()
                .with_plugin(RenderToWindow::from_config_path(display_config_path)?)
                .with_plugin(RenderPbr3D::default().with_skinning())
                .with_plugin(RenderSkybox::default()),
        )?;
    let mut game = Application::build(assets_dir, GameState::default())?.build(game_data)?;
    game.run();

    Ok(())
}

// fn initialize_camera(world: &mut World) {
//     let mut transform = Transform::default();
//     transform.set_translation_xyz(0.0, 0.0, 10.0);
//     world
//         .create_entity()
//         .with(Camera::standard_3d(1920.0, 1080.0))
//         .with(transform)
//         .build();
// }

// fn initialize_player(world: &mut World) {
//     let mesh = world.exec(|loader: AssetLoaderSystemData<'_, Mesh>| {
//         loader.load_from_data(
//             Shape::Sphere(100, 100)
//                 .generate::<(Vec<Position>, Vec<Normal>, Vec<Tangent>, Vec<TexCoord>)>(None)
//                 .into(),
//             (),
//         )
//     });

//     let material_defaults = world.read_resource::<MaterialDefaults>().0.clone();
//     let material = world.exec(|loader: AssetLoaderSystemData<'_, Material>| {
//         loader.load_from_data(
//             Material {
//                 ..material_defaults
//             },
//             (),
//         )
//     });

//     let mut transform = Transform::default();
//     transform.set_translation_xyz(0.0, 0.0, 0.0);
//     world
//         .create_entity()
//         .with(mesh)
//         .with(material)
//         .with(transform)
//         // .with(Type::Player)
//         // .with(Hp { hp: 100 })
//         .build();
// }

// fn initialize_scene(world: &mut World) {
//     let StateData { world, .. } = data;
//     world.exec(
//         |(loader, mut scene): (PrefabLoader<'_, ScenePrefabData>, Write<'_, Scene>)| {
//             scene.handle = Some(loader.load(
//                 "prefab/puffy_scene.ron",
//                 RonFormat,
//                 self.progress.as_mut().unwrap(),
//             ));
//         },
//     );
// }

// fn initialize_light(world: &mut World) {
//     let light: Light = PointLight {
//         intensity: 10.0,
//         color: Rgb::new(1.0, 1.0, 1.0),
//         ..PointLight::default()
//     }
//     .into();

//     let mut transform = Transform::default();
//     transform.set_translation_xyz(5.0, 5.0, 20.0);

//     world.create_entity().with(light).with(transform).build();
// }
