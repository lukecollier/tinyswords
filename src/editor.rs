use crate::{
    camera::MainCamera,
    characters::{Character, CharacterAssets, CharacterBundle},
    nav::{NavBundle, Navigation},
    world::{
        update_register_new_tile, LandMap, TileMap, WorldAssets, TILE_SIZE, TILE_VEC, WORLD_SIZE,
    },
    GameState,
};
use bevy::{prelude::*, render::camera::Viewport};
use bevy_asset_loader::prelude::*;
use bevy_egui::{
    egui::{self, text::LayoutJob},
    EguiContexts, EguiPlugin,
};

#[derive(AssetCollection, Resource)]
pub struct EditorAssets {
    // Tiles
    #[asset(path = "editor/grass_button.png")]
    grass: Handle<Image>,
    #[asset(path = "editor/sand_button.png")]
    sand: Handle<Image>,
    #[asset(path = "editor/steps_icon.png")]
    steps: Handle<Image>,
    #[asset(path = "editor/rock_icon.png")]
    rock: Handle<Image>,

    // Characters
    #[asset(path = "editor/pawn_icon.png")]
    pawn: Handle<Image>,
    #[asset(path = "editor/raider_icon.png")]
    raider: Handle<Image>,
}

#[derive(Eq, PartialEq)]
enum BrushType {
    Terrain(Terrain),
    Character(CharacterTemplate),
    None,
}

impl BrushType {
    fn is_terrain(&self) -> bool {
        match self {
            BrushType::Terrain(_) => true,
            _ => false,
        }
    }

    fn is_character(&self) -> bool {
        match self {
            BrushType::Character(_) => true,
            _ => false,
        }
    }
}

enum PaintShape {
    Square,
    Diamond,
}

#[derive(Resource)]
struct EditorOptions {
    show_terrain: bool,
    show_characters: bool,
    elevation: u8,
    brush_size: u8,
    brush_shape: PaintShape,
    brush: BrushType,
    is_mouse_on_ui: bool,
}

#[derive(Component)]
struct CharacterShadow;

#[derive(Component)]
struct EditorOnly;

impl Default for EditorOptions {
    fn default() -> Self {
        Self {
            show_terrain: false,
            show_characters: false,
            elevation: 0,
            brush_size: 1,
            brush_shape: PaintShape::Square,
            brush: BrushType::None,
            is_mouse_on_ui: false,
        }
    }
}

fn update_block_camera_move_egui(
    mut camera_q: Query<&mut MainCamera>,
    mut contexts: EguiContexts,
    mut options: ResMut<EditorOptions>,
) {
    for mut camera_config in camera_q.iter_mut() {
        if !contexts.ctx_mut().is_pointer_over_area() {
            camera_config.move_by_viewport_borders = true;
            options.is_mouse_on_ui = false;
        } else {
            camera_config.move_by_viewport_borders = false;
            options.is_mouse_on_ui = true;
        }
    }
}

fn update_place_character(
    mut cmds: Commands,
    window_q: Query<&Window>,
    mut camera_q: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut character_shadow_q: Query<(Entity, &mut Transform, &mut Sprite), With<CharacterShadow>>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    options: ResMut<EditorOptions>,
    pathing: Res<Navigation>,
    character_assets: Res<CharacterAssets>,
) {
    if !options.brush.is_character() || options.is_mouse_on_ui {
        for (entity, _, _) in &character_shadow_q {
            if let Some(response) = cmds.get_entity(entity) {
                response.despawn_recursive();
            }
        }
        return;
    }

    if let Ok(window) = window_q.get_single() {
        for (camera, camera_transform) in camera_q.iter_mut() {
            if let Some(cursor_pos) = window.cursor_position() {
                let mut top_offset_logical_pixels = 0.;
                if let Some(logical_rect) = camera.logical_viewport_rect() {
                    // note: Does not take into account that the viewport is offset by the navbar at
                    // the top
                    // so when we deal with the actual world we need to add the offset
                    top_offset_logical_pixels = window.height() - logical_rect.height();
                    if !logical_rect.contains(cursor_pos) {
                        for (entity, _, _) in &character_shadow_q {
                            if let Some(response) = cmds.get_entity(entity) {
                                response.despawn_recursive();
                            }
                        }
                        break;
                    };
                };
                if let Some(world_cursor_pos) =
                    camera.viewport_to_world_2d(camera_transform, cursor_pos)
                {
                    match character_shadow_q.get_single_mut() {
                        Ok((_, mut transform, mut sprite)) => {
                            *transform = Transform::from_translation(
                                (world_cursor_pos + Vec2::Y * top_offset_logical_pixels)
                                    .extend(transform.translation.z),
                            );
                            if pathing.is_walkable(transform.translation.truncate()) {
                                sprite.color = Color::rgba_linear_from_array([1., 1., 1., 0.5]);
                            } else {
                                sprite.color = Color::rgba_linear_from_array([1., 0., 0., 0.5]);
                            }
                        }
                        Err(bevy::ecs::query::QuerySingleError::NoEntities(_)) => {
                            match &options.brush {
                                BrushType::Character(template) => {
                                    let character =
                                        template.bundle(&character_assets, world_cursor_pos);
                                    cmds.spawn((
                                        character,
                                        template.clone(),
                                        CharacterShadow,
                                        EditorOnly,
                                    ));
                                }
                                _ => panic!("todo: represent the brush types as a AST"),
                            };
                        }
                        Err(bevy::ecs::query::QuerySingleError::MultipleEntities(_)) => {
                            for (entity, _, _) in &character_shadow_q {
                                if let Some(response) = cmds.get_entity(entity) {
                                    response.despawn_recursive();
                                }
                            }
                        }
                    };
                    if mouse_button.just_pressed(MouseButton::Left) {
                        for (entity, transform, mut sprite) in &mut character_shadow_q {
                            if pathing.is_walkable(
                                transform.translation.truncate()
                                    + Vec2::Y * top_offset_logical_pixels,
                            ) {
                                if let Some(mut response) = cmds.get_entity(entity) {
                                    sprite.color = Color::WHITE;
                                    response.remove::<CharacterShadow>();
                                    response.remove::<EditorOnly>();
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn update_place_terrain(
    mut cmds: Commands,
    window_q: Query<&Window>,
    mut camera_q: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    // todo: Find the tile that already exists and update
    mouse_button: Res<ButtonInput<MouseButton>>,
    options: ResMut<EditorOptions>,
    tile_map: ResMut<TileMap>,
    world_assets: Res<WorldAssets>,
    mut gizmos: Gizmos,
) {
    if !options.brush.is_terrain() || options.is_mouse_on_ui {
        return;
    }
    let Ok(window) = window_q.get_single() else {
        return;
    };
    for (camera, camera_transform) in camera_q.iter_mut() {
        let Some(cursor_pos) = window.cursor_position() else {
            return;
        };
        let mut top_offset_logical_pixels = 0.;
        if let Some(logical_rect) = camera.logical_viewport_rect() {
            top_offset_logical_pixels = window.height() - logical_rect.height();
            if !logical_rect.contains(cursor_pos) {
                break;
            };
        };
        let Some(world_cursor_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos)
        else {
            return;
        };
        let tile_pos = ((world_cursor_pos + Vec2::Y * top_offset_logical_pixels) / TILE_VEC)
            .floor()
            .as_u16vec2();
        let mut selected_elevation: &u8 =
            tile_map.get_elevation(tile_pos.x, tile_pos.y).unwrap_or(&0);

        // todo(improvement): Should be two squares, one at elevation 0 and one at the selected
        // elevation
        gizmos.rect_2d(
            tile_pos.as_vec2() * TILE_VEC + (TILE_VEC / 2.0)
                - (Vec2::Y * options.brush_size as f32) / 2.0,
            0.0,
            TILE_VEC * Vec2::new(1.0, 1.0 as f32) * options.brush_size as f32,
            Color::GREEN,
        );

        if mouse_button.pressed(MouseButton::Left) {
            let place_size = options.brush_size as u16;
            for x in 0..place_size {
                for y in 0..place_size {
                    let p_x = (tile_pos.x + x) as i32 - place_size as i32 / 2;
                    let p_y = (tile_pos.y + y) as i32 - place_size as i32 / 2;
                    if p_x >= 0 as i32
                        && p_y >= 0 as i32
                        && p_x < WORLD_SIZE.x as i32
                        && p_y < WORLD_SIZE.y as i32
                        && !tile_map.contains(p_x, p_y)
                    {
                        match options.brush {
                            BrushType::Terrain(_) if options.elevation > 0 => {
                                world_assets.spawn_empty(
                                    &mut cmds,
                                    p_x as u16,
                                    p_y as u16,
                                    options.elevation,
                                );
                            }
                            BrushType::Terrain(Terrain::Sand) if options.elevation == 0 => {
                                cmds.spawn(NavBundle::allowed(
                                    p_x as f32 * TILE_SIZE,
                                    p_y as f32 * TILE_SIZE,
                                    TILE_SIZE,
                                    TILE_SIZE,
                                ));
                                world_assets.spawn_sand(&mut cmds, p_x as u16, p_y as u16, 0);
                            }
                            BrushType::Terrain(Terrain::Grass) if options.elevation == 0 => {
                                cmds.spawn(NavBundle::allowed(
                                    p_x as f32 * TILE_SIZE,
                                    p_y as f32 * TILE_SIZE,
                                    TILE_SIZE,
                                    TILE_SIZE,
                                ));
                                world_assets.spawn_grass(&mut cmds, p_x as u16, p_y as u16, 0);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }
}

fn debug_nav_pathing(gizmos: Gizmos, navigation: Res<Navigation>) {
    navigation.debug(gizmos);
}

fn update_editor_menu(
    mut contexts: EguiContexts,
    mut options: ResMut<EditorOptions>,
    window_q: Query<&Window>,
    mut camera_q: Query<&mut Camera, With<MainCamera>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    use egui::*;
    let logical_height = TopBottomPanel::top("top_panel")
        .show(contexts.ctx_mut(), |ui| {
            menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open").clicked() {}
                    if ui.button("Save").clicked() {}
                })
                .response;
                let mut layout_job = LayoutJob::default();
                RichText::new("T").color(Color32::YELLOW).append_to(
                    &mut layout_job,
                    &ui.style(),
                    FontSelection::Default,
                    Align::Center,
                );
                RichText::new("errain")
                    .color(Color32::LIGHT_GRAY)
                    .append_to(
                        &mut layout_job,
                        &ui.style(),
                        FontSelection::Default,
                        Align::Center,
                    );
                if ui.button(layout_job).clicked() || keyboard_input.just_pressed(KeyCode::KeyT) {
                    options.show_terrain = !options.show_terrain;
                }

                let mut layout_job_play = LayoutJob::default();
                RichText::new("P").color(Color32::YELLOW).append_to(
                    &mut layout_job_play,
                    &ui.style(),
                    FontSelection::Default,
                    Align::Center,
                );
                RichText::new("lay").color(Color32::LIGHT_GRAY).append_to(
                    &mut layout_job_play,
                    &ui.style(),
                    FontSelection::Default,
                    Align::Center,
                );
                if ui.button(layout_job_play).clicked()
                    || keyboard_input.just_pressed(KeyCode::KeyP)
                {
                    next_state.set(GameState::InGame);
                }

                let mut layout_job_characters = LayoutJob::default();
                RichText::new("C").color(Color32::YELLOW).append_to(
                    &mut layout_job_characters,
                    &ui.style(),
                    FontSelection::Default,
                    Align::Center,
                );
                RichText::new("haracters")
                    .color(Color32::LIGHT_GRAY)
                    .append_to(
                        &mut layout_job_characters,
                        &ui.style(),
                        FontSelection::Default,
                        Align::Center,
                    );

                if ui.button(layout_job_characters).clicked()
                    || keyboard_input.just_pressed(KeyCode::KeyC)
                {
                    options.show_characters = !options.show_characters;
                }
            });
        })
        .response
        .rect
        .height();
    if let Ok(window) = window_q.get_single() {
        for mut camera in camera_q.iter_mut() {
            if let Some(scaling_factor) = camera.target_scaling_factor() {
                camera.viewport = Some(Viewport {
                    physical_position: UVec2::new(0, (logical_height * scaling_factor) as u32),
                    physical_size: UVec2::new(
                        (window.physical_width()) as u32,
                        (window.physical_height()) as u32
                            - (logical_height * scaling_factor) as u32,
                    ),
                    ..default()
                });
            }
        }
    }
}

#[derive(Component, Eq, PartialEq, Clone, Copy)]
enum Terrain {
    Grass,
    Sand,
    Rock,
    Steps,
}

#[derive(Component, Eq, PartialEq, Clone, Copy)]
enum CharacterTemplate {
    Pawn,
    Raider,
}

impl CharacterTemplate {
    fn bundle(&self, character_assets: &CharacterAssets, xy: Vec2) -> CharacterBundle {
        match self {
            CharacterTemplate::Pawn => character_assets.pawn(xy),
            CharacterTemplate::Raider => character_assets.raider(xy),
        }
    }
}

fn cleanup_entities_on_exit(mut cmds: Commands, cleanup_q: Query<Entity, With<EditorOnly>>) {
    for cleanup_entity in &cleanup_q {
        if let Some(found_entity) = cmds.get_entity(cleanup_entity) {
            found_entity.despawn_recursive();
        }
    }
}

fn save_scene(world: &mut World, mut store: ResMut<EditorWorld>) {
    let mut characters = world.query_filtered::<Entity, With<Character>>();
    let scene = DynamicSceneBuilder::from_world(world)
        .extract_entities(characters.iter(&world))
        .build();
    let type_registry = world.resource::<AppTypeRegistry>();
    let ron_scene = scene.serialize_ron(&type_registry);
}

fn store_on_exit(
    mut cmds: Commands,
    mut store: ResMut<EditorWorld>,
    characters_q: Query<(Entity, &CharacterTemplate, &GlobalTransform)>,
    assets: Res<CharacterAssets>,
) {
    let c = characters_q.iter().collect::<Vec<_>>();
    store.characters = c
        .into_iter()
        .map(|(_, c, t)| (c.bundle(&assets, t.translation().truncate()), c.clone()))
        .collect();
    for (character_entity, _, _) in &characters_q {
        if let Some(mut found_entity) = cmds.get_entity(character_entity) {
            found_entity.remove::<CharacterTemplate>();
        }
    }
}

fn cleanup_entities_on_enter(mut cmds: Commands, cleanup_q: Query<Entity, With<Character>>) {
    for cleanup_entity in &cleanup_q {
        if let Some(found_entity) = cmds.get_entity(cleanup_entity) {
            found_entity.despawn_recursive();
        }
    }
}

fn restore_on_enter(mut cmds: Commands, store: ResMut<EditorWorld>) {
    cmds.spawn_batch(store.characters.clone());
}

fn update_editor_ui(
    mut contexts: EguiContexts,
    assets: Res<EditorAssets>,
    mut options: ResMut<EditorOptions>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    use egui::*;

    if options.show_characters {
        let pawn_texture = contexts.add_image(assets.pawn.clone_weak());
        let raider_texture = contexts.add_image(assets.raider.clone_weak());
        egui::Window::new("Characters")
            .resizable(false)
            .movable(true)
            .collapsible(false)
            .title_bar(true)
            .show(contexts.ctx_mut(), |ui| {
                ui.heading("Knights");
                egui::Grid::new("character_editor")
                    .striped(true)
                    .show(ui, |ui| {
                        let pawn_image = egui::load::SizedTexture::new(pawn_texture, [32.0, 32.0]);
                        if ImageButton::new(pawn_image)
                            .selected(
                                options.brush == BrushType::Character(CharacterTemplate::Pawn),
                            )
                            .ui(ui)
                            .on_hover_text("pawn")
                            .clicked()
                        {
                            if options.brush == BrushType::Character(CharacterTemplate::Pawn) {
                                options.brush = BrushType::None;
                            } else {
                                options.brush = BrushType::Character(CharacterTemplate::Pawn);
                            }
                        };
                    });
                ui.separator();
                ui.heading("Goblins");
                let raider_image = egui::load::SizedTexture::new(raider_texture, [32.0, 32.0]);
                if ImageButton::new(raider_image)
                    .selected(options.brush == BrushType::Character(CharacterTemplate::Raider))
                    .ui(ui)
                    .on_hover_text("raider")
                    .clicked()
                {
                    if options.brush == BrushType::Character(CharacterTemplate::Raider) {
                        options.brush = BrushType::None;
                    } else {
                        options.brush = BrushType::Character(CharacterTemplate::Raider);
                    }
                };
            });
    }

    if options.show_terrain {
        let rock_texture = contexts.add_image(assets.rock.clone_weak());
        let sand_texture = contexts.add_image(assets.sand.clone_weak());
        let steps_texture = contexts.add_image(assets.steps.clone_weak());
        let grass_texture = contexts.add_image(assets.grass.clone_weak());
        egui::Window::new("Terrain")
            .resizable(false)
            .movable(true)
            .collapsible(false)
            .title_bar(true)
            .show(contexts.ctx_mut(), |ui| {
                egui::Grid::new("terrain_editor")
                    .striped(true)
                    .show(ui, |ui| {
                        let sand_image = egui::load::SizedTexture::new(sand_texture, [32.0, 32.0]);
                        if ImageButton::new(sand_image)
                            .selected(options.brush == BrushType::Terrain(Terrain::Sand))
                            .ui(ui)
                            .on_hover_text("sand")
                            .clicked()
                            || (options.show_terrain
                                && keyboard_input.just_pressed(KeyCode::Digit1))
                        {
                            if options.brush == BrushType::Terrain(Terrain::Sand) {
                                options.brush = BrushType::None;
                            } else {
                                options.brush = BrushType::Terrain(Terrain::Sand);
                            }
                        };
                        let grass_image =
                            egui::load::SizedTexture::new(grass_texture, [32.0, 32.0]);
                        if ImageButton::new(grass_image)
                            .selected(options.brush == BrushType::Terrain(Terrain::Grass))
                            .ui(ui)
                            .on_hover_text("grass")
                            .clicked()
                            || keyboard_input.just_pressed(KeyCode::Digit2)
                        {
                            if options.brush == BrushType::Terrain(Terrain::Grass) {
                                options.brush = BrushType::None;
                            } else {
                                options.brush = BrushType::Terrain(Terrain::Grass);
                            }
                        };
                        let rock_image = egui::load::SizedTexture::new(rock_texture, [32.0, 32.0]);
                        if ImageButton::new(rock_image)
                            .selected(options.brush == BrushType::Terrain(Terrain::Rock))
                            .ui(ui)
                            .on_hover_text("rocks")
                            .clicked()
                            || (options.show_terrain
                                && keyboard_input.just_pressed(KeyCode::Digit3))
                        {
                            if options.brush == BrushType::Terrain(Terrain::Rock) {
                                options.brush = BrushType::None;
                            } else {
                                options.brush = BrushType::Terrain(Terrain::Rock);
                            }
                        };
                        let steps_image =
                            egui::load::SizedTexture::new(steps_texture, [32.0, 32.0]);
                        if ImageButton::new(steps_image)
                            .selected(options.brush == BrushType::Terrain(Terrain::Steps))
                            .ui(ui)
                            .on_hover_text("steps_image")
                            .clicked()
                            || (options.show_terrain
                                && keyboard_input.just_pressed(KeyCode::Digit4))
                        {
                            if options.brush == BrushType::Terrain(Terrain::Steps) {
                                options.brush = BrushType::None;
                            } else {
                                options.brush = BrushType::Terrain(Terrain::Steps);
                            }
                        };
                    });
                ui.separator();
                let elevation_slider =
                    egui::Slider::new(&mut options.elevation, 0..=3).text("Elevation");
                ui.add(elevation_slider);
                ui.separator();
                let size_slider = egui::Slider::new(&mut options.brush_size, 1..=5)
                    .text("Brush Size")
                    .step_by(2.0);
                ui.add(size_slider);
            });
    }
}

// The editor world that can be saved or played?
#[derive(Resource, Default)]
pub struct EditorWorld {
    characters: Vec<(CharacterBundle, CharacterTemplate)>,
    characters_serialzied: Vec<u8>,
}

pub struct EditorPlugin<S: States> {
    state: S,
    loading_state: S,
}

impl<S: States> Plugin for EditorPlugin<S> {
    fn build(&self, app: &mut App) {
        app.configure_loading_state(
            // todo: Loading state should be configurable
            LoadingStateConfig::new(self.loading_state.clone()).load_collection::<EditorAssets>(),
        )
        .add_plugins(EguiPlugin)
        .init_resource::<EditorOptions>()
        .init_resource::<EditorWorld>()
        .add_systems(
            OnExit(self.state.clone()),
            (
                cleanup_entities_on_exit.before(store_on_exit),
                store_on_exit,
            ),
        )
        .add_systems(
            OnEnter(self.state.clone()),
            (
                cleanup_entities_on_enter.before(restore_on_enter),
                restore_on_enter,
            ),
        )
        .add_systems(
            Update,
            (
                update_editor_ui,
                update_editor_menu,
                update_place_terrain,
                // todo(improvement): I'd love the api to just be "spawn components" eventually
                update_place_terrain,
                update_place_character,
                update_block_camera_move_egui,
                debug_nav_pathing,
            )
                .run_if(in_state(self.state.clone())),
        );
    }
}

impl<S: States> EditorPlugin<S> {
    pub fn run_on_state(state: S, loading: S) -> Self {
        Self {
            state,
            loading_state: loading,
        }
    }
}
