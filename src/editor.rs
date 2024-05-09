use crate::{
    camera::MainCamera,
    world::{Elevation, PlaceLandTile, Tile, TILE_VEC, WORLD_SIZE},
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
    #[asset(path = "editor/grass_button.png")]
    grass: Handle<Image>,
    #[asset(path = "editor/sand_button.png")]
    sand: Handle<Image>,
}

#[derive(Eq, PartialEq)]
enum Placing {
    Sand,
    Grass,
    None,
}

#[derive(Resource)]
struct EditorOptions {
    show_terrain: bool,
    show_characters: bool,
    increase_elevation: bool,
    painting: Placing,
}

pub struct EditorPlugin<S: States> {
    state: S,
}

impl Default for EditorOptions {
    fn default() -> Self {
        Self {
            show_terrain: false,
            show_characters: false,
            increase_elevation: false,
            painting: Placing::None,
        }
    }
}

fn update_place_land(
    window_q: Query<&Window>,
    mut camera_q: Query<(&Camera, &GlobalTransform, &mut MainCamera)>,
    mut contexts: EguiContexts,
    tile_q: Query<(&mut Tile, &mut Elevation)>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    mut sender: EventWriter<PlaceLandTile>,
    mut options: ResMut<EditorOptions>,
    mut gizmos: Gizmos,
) {
    if let Ok(window) = window_q.get_single() {
        for (camera, camera_transform, mut camera_config) in camera_q.iter_mut() {
            if !contexts.ctx_mut().is_pointer_over_area() {
                camera_config.move_by_viewport_borders = true;
                if let Some(cursor_pos) = window.cursor_position() {
                    if let Some(physical_rect) = camera.logical_viewport_rect() {
                        if !physical_rect.contains(cursor_pos) {
                            break;
                        };
                    };
                    if let Some(world_cursor_pos) =
                        camera.viewport_to_world_2d(camera_transform, cursor_pos)
                    {
                        let tile_pos = (world_cursor_pos / TILE_VEC).floor().as_u16vec2();
                        let mut selected_elevation: u8 = 0;
                        for (tile, elevation) in &tile_q {
                            if tile.pos == tile_pos {
                                selected_elevation = elevation.0.max(selected_elevation);
                                break;
                            };
                        }

                        gizmos.rect_2d(
                            tile_pos.as_vec2() * TILE_VEC
                                + (TILE_VEC / 2.0)
                                + (Vec2::Y * TILE_VEC / 2.0) * selected_elevation as f32,
                            0.0,
                            TILE_VEC * Vec2::new(1.0, 1.0 * (selected_elevation + 1) as f32),
                            Color::GREEN,
                        );

                        let place_size = 3;
                        for x in 0..place_size {
                            for y in 0..place_size {
                                let p_x = (tile_pos.x + x) as i32 - place_size as i32 / 2;
                                let p_y = (tile_pos.y + y) as i32 - place_size as i32 / 2;
                                if mouse_button.pressed(MouseButton::Left)
                                    && p_x >= 0 as i32
                                    && p_x >= 0 as i32
                                    && p_x < WORLD_SIZE.x as i32
                                    && p_y < WORLD_SIZE.y as i32
                                {
                                    match options.painting {
                                        Placing::Sand => {
                                            sender.send(PlaceLandTile::sand(
                                                p_x as u16,
                                                p_y as u16,
                                                selected_elevation,
                                            ));
                                        }
                                        Placing::Grass => {}
                                        Placing::None => {}
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                camera_config.move_by_viewport_borders = false;
            }
        }
    }
}

fn update_editor_menu(
    mut contexts: EguiContexts,
    mut options: ResMut<EditorOptions>,
    window_q: Query<&Window>,
    mut camera_q: Query<&mut Camera, With<MainCamera>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    use egui::*;
    let logical_height = TopBottomPanel::top("top_panel")
        .show(contexts.ctx_mut(), |ui| {
            menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open").clicked() {
                        dbg!("open");
                    }
                    if ui.button("Save").clicked() {
                        dbg!("save");
                    }
                })
                .response;
                let mut layout_job = LayoutJob::default();
                let style = Style::default();
                RichText::new("T").color(Color32::YELLOW).append_to(
                    &mut layout_job,
                    &style,
                    FontSelection::Default,
                    Align::Center,
                );
                RichText::new("errain")
                    .color(Color32::LIGHT_GRAY)
                    .append_to(
                        &mut layout_job,
                        &style,
                        FontSelection::Default,
                        Align::Center,
                    );
                if ui.button(layout_job).clicked() || keyboard_input.just_pressed(KeyCode::KeyT) {
                    options.show_terrain = !options.show_terrain;
                }

                let mut layout_job_play = LayoutJob::default();
                let style = Style::default();
                RichText::new("P").color(Color32::YELLOW).append_to(
                    &mut layout_job_play,
                    &style,
                    FontSelection::Default,
                    Align::Center,
                );
                RichText::new("lay").color(Color32::LIGHT_GRAY).append_to(
                    &mut layout_job_play,
                    &style,
                    FontSelection::Default,
                    Align::Center,
                );
                if ui.button(layout_job_play).clicked()
                    || keyboard_input.just_pressed(KeyCode::KeyP)
                {
                    // todo: Change state to just game
                }

                let mut layout_job_characters = LayoutJob::default();
                let style = Style::default();
                RichText::new("C").color(Color32::YELLOW).append_to(
                    &mut layout_job_characters,
                    &style,
                    FontSelection::Default,
                    Align::Center,
                );
                RichText::new("haracters")
                    .color(Color32::LIGHT_GRAY)
                    .append_to(
                        &mut layout_job_characters,
                        &style,
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

fn update_editor_ui(
    mut contexts: EguiContexts,
    assets: Res<EditorAssets>,
    mut options: ResMut<EditorOptions>,
) {
    use egui::*;

    if options.show_characters {
        egui::Window::new("Characters")
            .resizable(false)
            .movable(true)
            .collapsible(false)
            .title_bar(false)
            .show(contexts.ctx_mut(), |ui| {
                ui.label("Characters");
                egui::Grid::new("character_editor")
                    .striped(true)
                    .show(ui, |ui| {});
            });
    }

    if options.show_terrain {
        let sand_texture = contexts.add_image(assets.sand.clone_weak());
        let grass_texture = contexts.add_image(assets.grass.clone_weak());
        egui::Window::new("Terrain")
            .resizable(false)
            .movable(true)
            .collapsible(false)
            .title_bar(false)
            .show(contexts.ctx_mut(), |ui| {
                ui.label("Terrain");
                egui::Grid::new("terrain_editor")
                    .striped(true)
                    .show(ui, |ui| {
                        let sand_image = egui::load::SizedTexture::new(sand_texture, [32.0, 32.0]);
                        if ImageButton::new(sand_image)
                            .selected(options.painting == Placing::Sand)
                            .ui(ui)
                            .on_hover_text("sand")
                            .clicked()
                        {
                            if options.painting == Placing::Sand {
                                options.painting = Placing::None;
                            } else {
                                options.painting = Placing::Sand;
                            }
                        };
                        let grass_image =
                            egui::load::SizedTexture::new(grass_texture, [32.0, 32.0]);
                        if ImageButton::new(grass_image)
                            .selected(options.painting == Placing::Grass)
                            .ui(ui)
                            .on_hover_text("grass")
                            .clicked()
                        {
                            if options.painting == Placing::Grass {
                                options.painting = Placing::None;
                            } else {
                                options.painting = Placing::Grass;
                            }
                        };
                    });
                ui.separator();
                ui.checkbox(&mut options.increase_elevation, "Increase Elevation");
            });
    }
}

impl<S: States> Plugin for EditorPlugin<S> {
    fn build(&self, app: &mut App) {
        app.configure_loading_state(
            LoadingStateConfig::new(GameState::AssetLoading).load_collection::<EditorAssets>(),
        )
        .add_plugins(EguiPlugin)
        .init_resource::<EditorOptions>()
        .add_systems(
            Update,
            (update_editor_ui, update_editor_menu, update_place_land)
                .run_if(in_state(self.state.clone())),
        );
    }
}

impl<S: States> EditorPlugin<S> {
    pub fn run_on_state(state: S) -> Self {
        Self { state }
    }
}
