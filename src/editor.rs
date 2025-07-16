use std::{fs::File, path::PathBuf};

use crate::{
    camera::MainCamera,
    characters::{Character, CharacterAssets},
    nav::Navigation,
    terrain::{TerrainTile, TerrainWorldDefault},
    InGameState,
};
use bevy::{
    color::palettes::css::GREEN, prelude::*, render::camera::Viewport, scene::InstanceId,
    state::state::FreelyMutableState, tasks::IoTaskPool, winit::WinitWindows,
};
use bevy_asset_loader::prelude::*;
use bevy_egui::{
    egui::{self, text::LayoutJob},
    EguiContexts, EguiPlugin,
};
use std::io::Write;

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
    Character(Character),
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
    file_path: Option<PathBuf>,
    show_terrain: bool,
    show_characters: bool,
    elevation: u8,
    brush_size: u8,
    brush_shape: PaintShape,
    brush: BrushType,
    is_mouse_on_ui: bool,
    scene: Handle<DynamicScene>,
    scene_instance_id: Option<InstanceId>,
}

impl Default for EditorOptions {
    fn default() -> Self {
        Self {
            file_path: None,
            show_terrain: false,
            show_characters: false,
            elevation: 0,
            brush_size: 1,
            brush_shape: PaintShape::Square,
            brush: BrushType::None,
            is_mouse_on_ui: false,
            scene: Handle::default(),
            scene_instance_id: None,
        }
    }
}

// get's serialized and maintained across edits
#[derive(Resource, Reflect, Default)]
#[reflect(Resource)]
struct EditorStore {
    last_editor_id: usize,
    undo_log: Vec<EditorActions>,
}

impl EditorStore {
    // used to maintain the undo log across scenes
    fn next_id(&mut self) -> EditorId {
        self.last_editor_id += 1;
        EditorId(self.last_editor_id)
    }
}

#[derive(Component)]
struct CharacterShadow;

#[derive(Component)]
struct EditorOnly;

// todo: This is messy, handles the first time we spawn characters before their part of a scene
// instance
#[derive(Component)]
struct CleanupCharacters;

#[derive(Component, Eq, PartialEq, Clone, Copy, Reflect, Debug)]
#[reflect(Component)]
struct EditorId(usize);

#[derive(Event, Reflect, Debug)]
struct EditorCommands {
    undo: bool,
    action: EditorActions,
}

impl EditorCommands {
    fn can_undo(action: EditorActions) -> Self {
        Self { undo: true, action }
    }

    fn cant_undo(action: EditorActions) -> Self {
        Self {
            undo: false,
            action,
        }
    }
}
// Allows for backtracking and forwarding
// as a hack we can push the opposite action to the undo history
// so if we placed a character, undoing would delete the character
// then we simply pop and send the event to our command
#[derive(Event, Reflect, Debug)]
enum EditorActions {
    CreateCharacter {
        position: Vec3,
        character: Character,
    },
    // The character deleted
    DeleteCharacter(EditorId),
    // for undo we send a command that will update the terrain
    UpdateTerrain {
        position: UVec2,
        new_terrain_type: Terrain,
    },
}

fn update_handle_editor_actions(
    mut cmds: Commands,
    mut ev_actions: EventReader<EditorCommands>,
    mut terrain: ResMut<TerrainWorldDefault>,
    mut store: ResMut<EditorStore>,
    editor_q: Query<(Entity, &EditorId)>,
    character_assets: Res<CharacterAssets>,
) {
    for ev in ev_actions.read() {
        match &ev.action {
            EditorActions::CreateCharacter {
                position,
                character,
            } => {
                let id = store.next_id();
                cmds.spawn((
                    *character,
                    character.animated_sprite(&character_assets),
                    CleanupCharacters,
                    id,
                    Transform::from_translation(*position),
                ));
                if ev.undo {
                    store
                        .undo_log
                        .push(EditorActions::DeleteCharacter(id.clone()));
                }
            }
            EditorActions::DeleteCharacter(id) => {
                let (entity, _) = editor_q
                    .iter()
                    .find(|(_, q_id)| *q_id == id)
                    .expect("couldn't find editor entity :(");
                cmds.entity(entity).despawn_recursive();
            }
            EditorActions::UpdateTerrain {
                position,
                new_terrain_type,
            } => {
                if let Some(prev_tile) = terrain.get_tile_from(&position) {
                    let prev_terrain = match prev_tile.terrain {
                        crate::terrain::Terrain::Sand => Terrain::Sand,
                        crate::terrain::Terrain::Grass => Terrain::Grass,
                        crate::terrain::Terrain::Water => Terrain::Water,
                    };
                    if ev.undo {
                        store.undo_log.push(EditorActions::UpdateTerrain {
                            position: *position,
                            new_terrain_type: prev_terrain,
                        });
                    }
                }
                match new_terrain_type {
                    Terrain::Grass => {
                        if let Ok(_) = terrain.set_to_grass(position) {
                            return;
                        } else {
                            error!("errored while updating sand");
                        };
                    }
                    Terrain::Water => {
                        if let Ok(_) = terrain.set_to_water(position) {
                            return;
                        } else {
                            error!("errored while updating sand");
                        };
                    }
                    Terrain::Sand => {
                        if let Ok(_) = terrain.set_to_sand(position) {
                            return;
                        } else {
                            error!("errored while updating sand");
                        };
                    }
                    Terrain::Rock => todo!(),
                    Terrain::Steps => todo!(),
                }
            }
            action => warn!("not yet implemented action used: {action:?}"),
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

//todo: We should use events so we can log every change made in the editor and rollback (or even
//more fun play a timelapse of the level being made!)
fn update_place_character(
    mut cmds: Commands,
    window_q: Query<&Window>,
    mut camera_q: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut character_shadow_q: Query<
        (Entity, &Character, &mut Transform, &mut Sprite),
        With<CharacterShadow>,
    >,
    mouse_button: Res<ButtonInput<MouseButton>>,
    options: ResMut<EditorOptions>,
    pathing: Res<Navigation>,
    character_assets: Res<CharacterAssets>,
    mut ev: EventWriter<EditorCommands>,
) {
    if !options.brush.is_character() || options.is_mouse_on_ui {
        for (entity, _, _, _) in &character_shadow_q {
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
                        for (entity, _, _, _) in &character_shadow_q {
                            if let Some(response) = cmds.get_entity(entity) {
                                response.despawn_recursive();
                            }
                        }
                        break;
                    };
                };
                if let Ok(world_cursor_pos) =
                    camera.viewport_to_world_2d(camera_transform, cursor_pos)
                {
                    match character_shadow_q.get_single_mut() {
                        Ok((_, _, mut transform, mut sprite)) => {
                            *transform = Transform::from_translation(
                                (world_cursor_pos + Vec2::Y * top_offset_logical_pixels)
                                    .extend(transform.translation.z),
                            );
                            if pathing.is_walkable(transform.translation.truncate()) {
                                sprite.color = Color::linear_rgba(1., 1., 1., 0.5);
                            } else {
                                sprite.color = Color::linear_rgba(1., 0., 0., 0.5);
                            }
                        }
                        Err(bevy::ecs::query::QuerySingleError::NoEntities(_)) => {
                            match &options.brush {
                                BrushType::Character(character) => {
                                    let animated_sprite =
                                        character.animated_sprite(&character_assets);
                                    cmds.spawn((
                                        Transform::from_translation(world_cursor_pos.extend(100.)),
                                        character.clone(),
                                        animated_sprite,
                                        CharacterShadow,
                                        EditorOnly,
                                    ));
                                }
                                _ => panic!("todo: represent the brush types as a AST"),
                            };
                        }
                        Err(bevy::ecs::query::QuerySingleError::MultipleEntities(_)) => {
                            for (entity, _, _, _) in &character_shadow_q {
                                if let Some(response) = cmds.get_entity(entity) {
                                    response.despawn_recursive();
                                }
                            }
                        }
                    };
                    if mouse_button.just_pressed(MouseButton::Left) {
                        for (_, template, transform, _) in &mut character_shadow_q {
                            if pathing.is_walkable(
                                transform.translation.truncate()
                                    + Vec2::Y * top_offset_logical_pixels,
                            ) {
                                let pos = (world_cursor_pos + Vec2::Y * top_offset_logical_pixels)
                                    .extend(100.);
                                ev.send(EditorCommands::can_undo(EditorActions::CreateCharacter {
                                    position: pos,
                                    character: template.clone(),
                                }));
                            }
                        }
                    }
                }
            }
        }
    }
}

fn zoom_scale(
    mut query_camera: Query<&mut OrthographicProjection, With<MainCamera>>,
    button: Res<ButtonInput<KeyCode>>,
) {
    let mut projection = query_camera.single_mut();
    // zoom in
    if button.just_pressed(KeyCode::Minus) {
        projection.scale /= 1.25;
    }
    // zoom out
    if button.just_pressed(KeyCode::Equal) {
        projection.scale *= 1.25;
    }
}

fn on_exit_make_tiles_white(mut tiles_q: Query<&mut Sprite, With<TerrainTile>>) {
    for mut sprite in tiles_q.iter_mut() {
        if sprite.color == GREEN.into() {
            sprite.color = Color::WHITE;
        }
    }
}

fn update_terrain_tile_picking(mut cmds: Commands, tiles_q: Query<Entity, Added<TerrainTile>>) {
    fn recolor_on<E>(
        color: Color,
    ) -> impl Fn(Trigger<E>, Query<&mut Sprite>, Res<State<InGameState>>)
    where
        E: Clone + Reflect,
    {
        move |ev, mut sprites, state| {
            let state = state.get();
            if *state != InGameState::InEditor {
                return;
            }
            let Ok(mut sprite) = sprites.get_mut(ev.entity()) else {
                return;
            };
            sprite.color = color;
        }
    }

    fn on_click() -> impl Fn(
        Trigger<Pointer<Down>>,
        Res<TerrainWorldDefault>,
        EventWriter<EditorCommands>,
        Query<&GlobalTransform>,
        Res<EditorOptions>,
        Res<State<InGameState>>,
    ) {
        move |ev, terrain, mut actions, global_transform_q, options, state| {
            let state = state.get();
            if *state != InGameState::InEditor || !options.brush.is_terrain() {
                return;
            }
            let Ok(tile_transform) = global_transform_q.get(ev.entity()) else {
                return;
            };
            let Some(terrain_pos) = terrain.coords_to_world(&tile_transform.translation().xy())
            else {
                return;
            };
            match options.brush {
                BrushType::Terrain(terrain_type) => {
                    actions.send(EditorCommands::can_undo(EditorActions::UpdateTerrain {
                        position: terrain_pos,
                        new_terrain_type: terrain_type,
                    }));
                }
                _ => (),
            }
        }
    }

    fn on_move() -> impl Fn(
        Trigger<Pointer<Over>>,
        Res<TerrainWorldDefault>,
        EventWriter<EditorCommands>,
        Query<&GlobalTransform>,
        Res<ButtonInput<MouseButton>>,
        Res<EditorOptions>,
        Res<State<InGameState>>,
    ) {
        move |ev, terrain, mut actions, global_transform_q, button, options, state| {
            let state = state.get();
            if *state != InGameState::InEditor
                || !button.pressed(MouseButton::Left)
                || !options.brush.is_terrain()
            {
                return;
            }
            let Ok(tile_transform) = global_transform_q.get(ev.entity()) else {
                return;
            };
            let Some(terrain_pos) = terrain.coords_to_world(&tile_transform.translation().xy())
            else {
                return;
            };
            match options.brush {
                BrushType::Terrain(terrain_type) => {
                    actions.send(EditorCommands::can_undo(EditorActions::UpdateTerrain {
                        position: terrain_pos,
                        new_terrain_type: terrain_type,
                    }));
                }
                _ => (),
            }
            // sprite.color = color;
        }
    }
    for entity in &tiles_q {
        cmds.entity(entity)
            .observe(recolor_on::<Pointer<Over>>(GREEN.into()))
            .observe(recolor_on::<Pointer<Out>>(Color::WHITE))
            .observe(on_click())
            .observe(on_move());
    }
    // we can use this entity for visual elements before writing any changes back to our terrain
    // world
}

fn debug_nav_pathing(gizmos: Gizmos, navigation: Res<Navigation>) {
    navigation.debug(gizmos);
}

// todo: For redo and undo we basicaly need a stack (vec) of entities, get's more difficult with
// tiles as they're actually a resource. We could change terrain to update itself based of the
// entities though :think:
fn update_editor_menu(
    mut contexts: EguiContexts,
    mut options: ResMut<EditorOptions>,
    mut store: ResMut<EditorStore>,
    window_q: Query<&Window>,
    mut camera_q: Query<&mut Camera, With<MainCamera>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut next_ingame_state: ResMut<NextState<InGameState>>,
    mut ev: EventWriter<EditorCommands>,
    // fixes rfd forcing running on the main thread
    mut _windows: NonSend<WinitWindows>,
) {
    use egui::*;
    let logical_height = TopBottomPanel::top("top_panel")
        .show(contexts.ctx_mut(), |ui| {
            menu::bar(ui, |ui| {
                let mut layout_job = LayoutJob::default();
                RichText::new("O").color(Color32::YELLOW).append_to(
                    &mut layout_job,
                    &ui.style(),
                    FontSelection::Default,
                    Align::Center,
                );
                RichText::new("pen").color(Color32::LIGHT_GRAY).append_to(
                    &mut layout_job,
                    &ui.style(),
                    FontSelection::Default,
                    Align::Center,
                );
                if ui.button(layout_job).clicked()
                    || (keyboard_input.just_pressed(KeyCode::KeyO)
                        && keyboard_input.pressed(KeyCode::ControlLeft))
                {
                    if let Some(path) = rfd::FileDialog::new().pick_file() {
                        // todo: this should be done via the loading state
                        options.file_path = Some(path.clone());
                        next_ingame_state.set(InGameState::Loading);
                    }
                }
                let mut layout_job = LayoutJob::default();
                RichText::new("S").color(Color32::YELLOW).append_to(
                    &mut layout_job,
                    &ui.style(),
                    FontSelection::Default,
                    Align::Center,
                );
                RichText::new("ave").color(Color32::LIGHT_GRAY).append_to(
                    &mut layout_job,
                    &ui.style(),
                    FontSelection::Default,
                    Align::Center,
                );
                let save_button = egui::Button::new(layout_job);
                let enabled = ui.add_enabled(options.file_path.is_some(), save_button);
                if enabled.clicked() {
                    next_ingame_state.set(InGameState::Saving);
                }
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
                    next_ingame_state.set(InGameState::Running);
                }

                let mut undo_layout = LayoutJob::default();
                RichText::new("U").color(Color32::YELLOW).append_to(
                    &mut undo_layout,
                    &ui.style(),
                    FontSelection::Default,
                    Align::Center,
                );
                RichText::new("undo").color(Color32::LIGHT_GRAY).append_to(
                    &mut undo_layout,
                    &ui.style(),
                    FontSelection::Default,
                    Align::Center,
                );
                if ui.button(undo_layout).clicked() || keyboard_input.just_pressed(KeyCode::KeyU) {
                    if let Some(undo_entry) = store.undo_log.pop() {
                        ev.send(EditorCommands::cant_undo(undo_entry));
                    }
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

/**
 * Resets the camera to take up the full window
 */
fn on_exit_camera_full_window(
    window_q: Query<&Window>,
    mut camera_q: Query<&mut Camera, With<MainCamera>>,
) {
    if let Ok(window) = window_q.get_single() {
        for mut camera in camera_q.iter_mut() {
            camera.viewport = Some(Viewport {
                physical_position: UVec2::new(0, 0),
                physical_size: UVec2::new(window.physical_width(), window.physical_height()),
                ..default()
            });
        }
    }
}

#[derive(Component, Reflect, Eq, PartialEq, Clone, Copy, Debug)]
enum Terrain {
    Grass,
    Water,
    Sand,
    Rock,
    Steps,
}

// todo: The first timne we spawn characters they don't belong to a scene :(
fn despawn_characters(mut cmds: Commands, q: Query<Entity, With<Character>>) {
    for entity in &q {
        cmds.entity(entity).despawn_recursive();
    }
}

fn cleanup_entities_on_enter(mut scene_spawner: ResMut<SceneSpawner>, options: Res<EditorOptions>) {
    if let Some(id) = options.scene_instance_id {
        scene_spawner.despawn_instance(id);
    }
}

fn cleanup_entities_on_exit(mut cmds: Commands, cleanup_q: Query<Entity, With<EditorOnly>>) {
    for cleanup_entity in &cleanup_q {
        if let Some(found_entity) = cmds.get_entity(cleanup_entity) {
            found_entity.despawn_recursive();
        }
    }
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
                            .selected(options.brush == BrushType::Character(Character::Pawn))
                            .ui(ui)
                            .on_hover_text("pawn")
                            .clicked()
                        {
                            if options.brush == BrushType::Character(Character::Pawn) {
                                options.brush = BrushType::None;
                            } else {
                                options.brush = BrushType::Character(Character::Pawn);
                            }
                        };
                    });
                ui.separator();
                ui.heading("Goblins");
                let raider_image = egui::load::SizedTexture::new(raider_texture, [32.0, 32.0]);
                if ImageButton::new(raider_image)
                    .selected(options.brush == BrushType::Character(Character::Raider))
                    .ui(ui)
                    .on_hover_text("raider")
                    .clicked()
                {
                    if options.brush == BrushType::Character(Character::Raider) {
                        options.brush = BrushType::None;
                    } else {
                        options.brush = BrushType::Character(Character::Raider);
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

fn save_scene(world: &mut World) {
    let mut characters = world.query_filtered::<Entity, (With<Character>, With<Transform>)>();
    let scene = DynamicSceneBuilder::from_world(world)
        .deny_all_components()
        .deny_all_resources()
        .allow_resource::<TerrainWorldDefault>()
        .allow_component::<Character>()
        .allow_component::<EditorId>()
        .allow_component::<Transform>()
        .extract_entities(characters.iter(&world))
        .extract_resources()
        .build();
    let type_registry = world.resource::<AppTypeRegistry>().clone();
    let type_registry = type_registry.read();
    let serialized_scene = scene.serialize(&type_registry).unwrap();
    let file_path = world
        .get_resource::<EditorOptions>()
        .unwrap()
        .file_path
        .clone()
        .unwrap();
    #[cfg(not(target_arch = "wasm32"))]
    IoTaskPool::get()
        .spawn(async move {
            // Write the scene RON data to file
            File::create(file_path)
                .and_then(|mut file| file.write(serialized_scene.as_bytes()))
                .expect("Error while writing scene to file");
        })
        .detach();
}

fn change_state_to_editor(mut next_ingame_state: ResMut<NextState<InGameState>>) {
    next_ingame_state.set(InGameState::InEditor);
}

fn store_scene(world: &mut World) {
    let mut characters = world.query_filtered::<Entity, (With<Character>, With<Transform>)>();
    let scene = DynamicSceneBuilder::from_world(world)
        .deny_all_components()
        .deny_all_resources()
        .allow_resource::<TerrainWorldDefault>()
        .allow_resource::<EditorStore>()
        .allow_component::<Character>()
        .allow_component::<EditorId>()
        .allow_component::<Transform>()
        .extract_entities(characters.iter(&world))
        .extract_resources()
        .build();
    let mut dynamic_scenes = world.get_resource_mut::<Assets<DynamicScene>>().unwrap();
    let handle = dynamic_scenes.add(scene);
    let mut editor_options = world.get_resource_mut::<EditorOptions>().unwrap();
    editor_options.scene = handle;
}

fn scene_from_file_into_memory(mut options: ResMut<EditorOptions>, asset_server: Res<AssetServer>) {
    let path = options.file_path.clone().unwrap();
    let scene_handle: Handle<DynamicScene> = asset_server.load(path.clone());
    options.scene = scene_handle;
}

fn load_scene_from_memory(
    mut options: ResMut<EditorOptions>,
    mut scene_spawner: ResMut<SceneSpawner>,
) {
    let instance_id = scene_spawner.spawn_dynamic(options.scene.clone());
    options.scene_instance_id = Some(instance_id);
}

fn update_nav_data(terrain_world: Res<TerrainWorldDefault>, mut navigation: ResMut<Navigation>) {
    if terrain_world.is_changed() {
        navigation.rebuild_from_terrain(&terrain_world);
        // do something
    }
}

pub struct EditorPlugin<S: States, L: States> {
    state: S,
    loading_state: L,
}

impl<S: States + FreelyMutableState, L: States + FreelyMutableState> Plugin for EditorPlugin<S, L> {
    fn build(&self, app: &mut App) {
        app.configure_loading_state(
            LoadingStateConfig::new(self.loading_state.clone()).load_collection::<EditorAssets>(),
        )
        .register_type::<Transform>()
        .register_type::<EditorId>()
        .register_type::<EditorStore>()
        .init_resource::<EditorOptions>()
        .init_resource::<EditorStore>()
        .add_event::<EditorCommands>()
        .add_plugins(EguiPlugin)
        .add_systems(
            OnEnter(InGameState::Saving),
            (save_scene, change_state_to_editor).chain(),
        )
        .add_systems(
            OnEnter(InGameState::Loading),
            (scene_from_file_into_memory, change_state_to_editor).chain(),
        )
        .add_systems(
            OnEnter(self.state.clone()),
            (
                cleanup_entities_on_enter,
                despawn_characters,
                load_scene_from_memory,
            )
                .chain(),
        )
        .add_systems(
            Update,
            (
                update_editor_ui,
                update_editor_menu,
                update_nav_data,
                update_handle_editor_actions,
                update_terrain_tile_picking,
                zoom_scale,
                update_place_character,
                update_block_camera_move_egui,
                debug_nav_pathing,
            )
                .run_if(in_state(self.state.clone())),
        )
        .add_systems(
            OnExit(self.state.clone()),
            (
                cleanup_entities_on_exit,
                store_scene,
                on_exit_camera_full_window,
                on_exit_make_tiles_white,
            )
                .chain(),
        );
    }
}

impl<S: States, L: States> EditorPlugin<S, L> {
    pub fn run_on_state(state: S, loading: L) -> Self {
        Self {
            state,
            loading_state: loading,
        }
    }
}
