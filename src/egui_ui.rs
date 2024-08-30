// mod app;

use egui::{
    self,
    emath::{self, RectTransform},
    pos2, Color32, Rgba, Shape, Widget,
};
// EguiContexts, EguiPlugin,
use egui_plot::{Legend, Line, Plot};
use log::info;
use macroquad::color::{GRAY, GREEN, ORANGE, PURPLE, RED};

use crate::{
    consts::{ALL_INSTRUMENTS, BEATS_PER_LOOP},
    events::Events,
    get_hits_from_nth_loop,
    score::{
        compute_accuracy_of_single_hit, compute_loop_performance_for_voice,
        get_user_hit_timings_by_instrument, Accuracy, MISS_MARGIN,
    },
    voices::{Instrument, Voices},
    UserHit,
};

pub const GRID_ROWS: usize = 10;
pub const GRID_COLS: usize = 16;

// This resource holds information about the game:
pub struct UIState {
    selector_vec: Vec<String>,
    selected_idx: usize,

    is_playing: bool,
    bpm: f32,
    is_metronome_enabled: bool,
    volume_metronome: f32,
    volume_target_notes: f32,

    // audio
    current_loop: usize, // nth loop
    current_beat: f32,

    enabled_beats: [[bool; GRID_COLS]; GRID_ROWS],

    latency_offset_ms: f32,

    user_hits: Vec<UserHit>,
    desired_hits: Voices,
}

impl Default for UIState {
    fn default() -> Self {
        Self {
            // Example stuff:
            is_playing: false,

            selector_vec: vec![
                // TODO: String::from() vs .. "".to_owned() ?
                String::from("Rock"),
                String::from("Breakbeat"),
                String::from("Samba"),
            ],
            selected_idx: 0,

            current_loop: 2,
            current_beat: 2.3,

            bpm: 120.,

            is_metronome_enabled: false,
            volume_metronome: 0.75,
            volume_target_notes: 0.75,

            latency_offset_ms: 100.,

            enabled_beats: [[false; GRID_COLS]; GRID_ROWS],

            user_hits: vec![],
            desired_hits: Voices::new(),
        }
    }
}

impl UIState {
    // TODO: rename related to choosing a loop
    pub fn selector_vec(mut self, selector_vec: &Vec<String>) -> Self {
        self.selector_vec = selector_vec.clone();
        self
    }

    pub fn set_selected_idx(&mut self, idx: usize) {
        self.selected_idx = idx;
    }

    pub fn set_is_playing(&mut self, is_playing: bool) {
        self.is_playing = is_playing;
    }

    pub fn set_current_beat(&mut self, beat: f64) {
        self.current_beat = beat as f32;
    }

    pub fn set_current_loop(&mut self, val: usize) {
        self.current_loop = val;
    }

    pub fn set_enabled_beats(&mut self, voices: &Voices) {
        self.enabled_beats = voices.to_enabled_beats();
    }

    pub fn set_bpm(&mut self, bpm: f32) {
        self.bpm = bpm;
    }

    pub fn set_latency_offset(&mut self, offset: f32) {
        self.latency_offset_ms = offset;
    }

    pub fn set_user_hits(&mut self, hits: &Vec<UserHit>) {
        self.user_hits = hits.clone();
    }

    pub fn set_desired_hits(&mut self, voices: &Voices) {
        self.desired_hits = voices.clone();
    }
}

pub fn draw_ui(ctx: &egui_macroquad::egui::Context, ui_state: &UIState, events: &mut Vec<Events>) {
    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        // The top panel is often a good place for a menu bar:

        egui::menu::bar(ui, |ui| {
            // NOTE: no File->Quit on web pages!
            let is_web = cfg!(target_arch = "wasm32");
            if !is_web {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        // ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        println!("quit! but with earlier EGui..");
                    }
                });
                ui.add_space(16.0);
            }
            ui.add(egui::Label::new("BPM"));

            let mut local_bpm = ui_state.bpm;
            let bpm_slider = egui::Slider::new(&mut local_bpm, 40.0..=240.0);
            let bpm_slider_resp = bpm_slider.ui(ui);
            if bpm_slider_resp.changed() {
                events.push(Events::SetBPM(local_bpm as f64));
            }
            if ui.button("-").clicked() {
                events.push(Events::ChangeBPM { delta: -1. });
            }
            if ui.button("+").clicked() {
                events.push(Events::ChangeBPM { delta: 1. });
            }

            ui.separator();

            ui.add(
                // egui::ProgressBar::new(game_state.progress)
                egui::ProgressBar::new(ui_state.current_beat / BEATS_PER_LOOP as f32)
                    // .fill(Color32::BROWN)
                    .show_percentage(),
            );
        });
    });

    egui::SidePanel::left("left_panel")
        .resizable(true)
        .default_width(150.0)
        .width_range(80.0..=240.0)
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("Left Panel");
            });

            let button_text = match ui_state.is_playing {
                true => "Pause",
                false => "Play",
            };
            if ui.button(button_text).clicked() {
                events.push(Events::Pause);
            }

            ui.separator();

            ui.add(egui::Label::new("**Volume**"));
            ui.add(egui::Label::new("Metronome"));
            // TODO
            // ui.add(egui::Slider::new(&mut ui_state.volume_metronome, 0.0..=1.0));
            ui.add(egui::Label::new("Target Notes"));
            // TODO
            // ui.add(egui::Slider::new(
            //     &mut ui_state.volume_target_notes,
            //     0.0..=1.0,
            // ));

            ui.separator();

            ui.add(egui::Label::new("**Loop Status**"));
            ui.add(egui::Label::new("Current Loop"));
            ui.add(egui::Label::new(format!("{}", ui_state.current_loop)));
            ui.add(egui::Label::new("Current Beat"));
            ui.add(egui::Label::new(format!("{}", ui_state.current_beat)));

            ui.separator();

            gold_mode(ui);
        });

    egui::SidePanel::right("right_panel")
        .resizable(true)
        .default_width(150.0)
        .width_range(80.0..=240.0)
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("Right Panel");
            });

            egui::ComboBox::from_label("Choose Loop")
                .selected_text(format!("{}", &ui_state.selector_vec[ui_state.selected_idx]))
                .show_ui(ui, |ui| {
                    for i in 0..ui_state.selector_vec.len() {
                        let mut current_value = &ui_state.selector_vec[i];
                        let value = ui.selectable_value(
                            &mut current_value,
                            &ui_state.selector_vec[ui_state.selected_idx],
                            &ui_state.selector_vec[i],
                        );
                        if value.clicked() {
                            // TODO: handle with event
                            // ui_state.selected_idx = i;
                            // TODO: load the relevant loop's data
                            events.push(Events::ChangeLoop(i));
                        }
                    }
                });

            ui.separator();

            ui.group(|ui| {
                ui.add(egui::Label::new("Latency Offset (ms)"));
                ui.label(format!("{:?}", ui_state.latency_offset_ms));
                // TODO
                // ui.add(egui::Slider::new(
                //     &mut ui_state.latency_offset,
                //     -1000.0..=1000.0,
                // ));
                if ui.button("-").clicked() {
                    events.push(Events::SetAudioLatency { delta: -5. });
                    // ui_state.latency_offset -= 5.;
                }
                if ui.button("+").clicked() {
                    events.push(Events::SetAudioLatency { delta: 5. });
                }
            });

            ui.separator();
            egui::widgets::global_dark_light_mode_buttons(ui);

            ui.separator();

            // TODO: link to macroix github repo
            ui.add(egui::github_link_file!(
                "https://github.com/emilk/eframe_template/blob/main/",
                "Source code."
            ));

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                powered_by_egui_and_eframe(ui);
                egui::warn_if_debug_build(ui);
            });
        });

    egui::CentralPanel::default().show(ctx, |ui| {
        // The central panel the region left after adding TopPanel's and SidePanel's
        ui.heading("Macroix");

        draw_beat_grid(ui_state, ui, events);
    });
}

const VIRTUAL_WIDTH: f32 = 800.;
const VIRTUAL_HEIGHT: f32 = 1000.;
const WIDTH_SCALE: f32 = VIRTUAL_WIDTH / GRID_COLS as f32;
const HEIGHT_SCALE: f32 = VIRTUAL_HEIGHT / GRID_ROWS as f32;

fn draw_beat_grid(ui_state: &UIState, ui: &mut egui::Ui, events: &mut Vec<Events>) {
    let (response, painter) = ui.allocate_painter(
        egui::Vec2::new(ui.available_width(), ui.available_height()),
        egui::Sense::hover(),
    );

    let to_screen = emath::RectTransform::from_to(
        egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::Vec2::new(VIRTUAL_WIDTH, VIRTUAL_HEIGHT),
        ),
        response.rect,
    );
    let from_screen = emath::RectTransform::from_to(
        response.rect,
        egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::Vec2::new(VIRTUAL_WIDTH, VIRTUAL_HEIGHT),
        ),
    );

    // capture mouse clicks and toggle relevant beat
    ui.input(|i| {
        for event in &i.raw.events {
            match event {
                // TODO: what is this syntax
                egui::Event::PointerButton {
                    pos, pressed: true, ..
                } => {
                    // check if click is within the beat grid's bounds
                    if !response.rect.contains(*pos) {
                        continue;
                    }

                    // Translate to (row, col)
                    let tpos = from_screen.transform_pos(*pos);
                    let row = (tpos.y * GRID_ROWS as f32 / VIRTUAL_HEIGHT) as usize;
                    let col = (tpos.x * GRID_COLS as f32 / VIRTUAL_WIDTH) as usize;
                    info!(
                        "click at position = {:?} [[tpos = {:?}]] (row={:?}, col={:?})",
                        pos, tpos, row, col,
                    );
                    events.push(Events::ToggleBeat {
                        row: row as f64,
                        beat: col as f64,
                    });
                }
                _ => (),
            }
        }
    });

    let beat_fill_color = if ui.visuals().dark_mode {
        Color32::from_rgba_premultiplied(200, 200, 200, 128)
    } else {
        Color32::from_rgba_premultiplied(50, 50, 50, 128)
    };

    let mut shapes = vec![];
    for row in 0..GRID_ROWS {
        for col in 0..GRID_COLS {
            let t_rect = rect_for_col_row(col, row, to_screen);

            if col == 0 {
                let name = match ALL_INSTRUMENTS[row] {
                    Instrument::ClosedHihat => "Hi-hat",
                    Instrument::Snare => "Snare",
                    Instrument::Kick => "Kick",
                    Instrument::OpenHihat => "Open Hi-hat",
                    Instrument::Ride => "Ride",
                    Instrument::Crash => "Crash",
                    Instrument::Tom1 => "Tom1 (High)",
                    Instrument::Tom2 => "Tom2 (Med)",
                    Instrument::Tom3 => "Tom3 (Low)",
                    Instrument::PedalHiHat => "Pedal Hi-hat",
                };
                let label = egui::Label::new(name);
                ui.put(
                    t_rect,
                    // egui::Rect::from_min_max(t_rect.left_top(), t_rect.size().to_pos2()),
                    label,
                );
            }

            // if this beat is enabled (row is instrument, col is beat)..
            if ui_state.enabled_beats[row][col] {
                let shape =
                    egui::Shape::rect_filled(t_rect, egui::Rounding::default(), beat_fill_color);
                shapes.push(shape)
            }

            let shape = egui::Shape::rect_stroke(
                t_rect,
                egui::Rounding::default(),
                egui::Stroke::new(2., Color32::DARK_GRAY),
            );
            shapes.push(shape);
        }
    }

    // Draw User Hits
    draw_user_hits(ui_state, to_screen, &mut shapes);

    // Draw Note Successes
    let loop_last_completed_beat = ui_state.current_beat - MISS_MARGIN as f32;
    let current_loop_hits = get_hits_from_nth_loop(&ui_state.user_hits, ui_state.current_loop);
    draw_note_successes(
        &current_loop_hits,
        &ui_state.desired_hits,
        (ui_state.latency_offset_ms / 1000.) as f64,
        loop_last_completed_beat as f64,
        to_screen,
        &mut shapes,
    );

    // ---

    draw_current_beat(ui_state.current_beat, to_screen, ui, &mut shapes);

    // render them
    painter.extend(shapes);
}

fn rect_for_col_row(col: usize, row: usize, to_screen: RectTransform) -> egui::Rect {
    let base_pos = pos2(col as f32 * WIDTH_SCALE, row as f32 * HEIGHT_SCALE);

    // TODO: fix scaling to always draw a nicer looking square based grid
    let t_rect = to_screen.transform_rect(egui::Rect {
        min: base_pos,
        max: base_pos + egui::Vec2::new(WIDTH_SCALE * 0.95, HEIGHT_SCALE * 0.95),
    });
    t_rect
}

fn draw_current_beat(
    current_beat: f32,
    to_screen: RectTransform,
    ui: &mut egui::Ui,
    shapes: &mut Vec<Shape>,
) {
    let base_pos = pos2((current_beat / BEATS_PER_LOOP as f32) * VIRTUAL_WIDTH, 0.);
    let t_rect = to_screen.transform_rect(egui::Rect {
        min: base_pos,
        max: base_pos + egui::Vec2::new(2., VIRTUAL_HEIGHT),
    });

    let bar_color = if ui.visuals().dark_mode {
        Color32::YELLOW
    } else {
        Color32::BLUE
    };
    let shape = egui::Shape::rect_filled(t_rect, egui::Rounding::default(), bar_color);
    shapes.push(shape);
}

fn draw_user_hits(ui_state: &UIState, to_screen: RectTransform, shapes: &mut Vec<Shape>) {
    for (instrument_idx, instrument) in ALL_INSTRUMENTS.iter().enumerate() {
        let user_notes = get_user_hit_timings_by_instrument(&ui_state.user_hits, *instrument);
        let desired_notes = ui_state.desired_hits.get_instrument_beats(instrument);
        for note in user_notes.iter() {
            draw_user_hit(
                *note,
                instrument_idx,
                ui_state.latency_offset_ms as f64,
                desired_notes,
                to_screen,
                shapes,
            );
        }
    }
}

fn draw_user_hit(
    user_beat: f64,
    row: usize,
    audio_latency_ms: f64,
    desired_hits: &Vec<f64>,
    to_screen: RectTransform,
    shapes: &mut Vec<Shape>,
) {
    // TODO: Want audio latency in terms of BEATS
    let user_beat_with_latency = user_beat + (audio_latency_ms / 1000.);

    let (acc, is_next_loop) = compute_accuracy_of_single_hit(user_beat_with_latency, desired_hits);

    // with audio latency and is_next_loop
    // TODO(bug): hit a note on every beat of 16. Then toggle on and off a note on only beat 1 for that instrument. it causes buggy display of hit timings where the 2nd half (beats 9-16) aren't shown .. bercause it's closer to beat 1 than any other beat, I guess?.
    // TODO(ui): can't see "before" hits because there's no space to left anymore
    let x = if is_next_loop {
        ((user_beat_with_latency as f32 - BEATS_PER_LOOP as f32) / BEATS_PER_LOOP as f32)
            * VIRTUAL_WIDTH
    } else {
        (user_beat_with_latency as f32 / BEATS_PER_LOOP as f32) * VIRTUAL_WIDTH
    };

    let base_pos = pos2(x as f32, row as f32 * HEIGHT_SCALE);
    let t_rect = to_screen.transform_rect(egui::Rect {
        min: base_pos,
        max: base_pos + egui::Vec2::new(2., HEIGHT_SCALE * 0.95),
    });

    info!("Drawing user hit at {:?}", t_rect);

    let bar_color = match acc {
        Accuracy::Early => ORANGE,
        Accuracy::Late => PURPLE,
        Accuracy::Correct => GREEN,
        Accuracy::Miss => RED,
        Accuracy::Unknown => GRAY,
    };
    let bar_color_32 = Color32::from_rgb(
        (bar_color.r * 256.) as u8,
        (bar_color.g * 256.) as u8,
        (bar_color.b * 256.) as u8,
    );

    let shape = egui::Shape::rect_filled(t_rect, egui::Rounding::default(), bar_color_32);
    shapes.push(shape);
}

fn draw_note_successes(
    user_hits: &Vec<UserHit>,
    desired_hits: &Voices,
    audio_latency: f64,
    loop_current_beat: f64,
    to_screen: RectTransform,
    shapes: &mut Vec<Shape>,
) {
    for (instrument_idx, instrument) in ALL_INSTRUMENTS.iter().enumerate() {
        let actual = get_user_hit_timings_by_instrument(user_hits, *instrument);
        // add audio_latency to each note
        let actual_w_latency = actual
            .iter()
            .map(|note| note + audio_latency)
            .collect::<Vec<f64>>();

        let desired = desired_hits.get_instrument_beats(instrument);

        let loop_perf =
            compute_loop_performance_for_voice(&actual_w_latency, &desired, loop_current_beat);
        for (note_idx, note) in desired.iter().enumerate() {
            let shape = note_success_shape(*note, instrument_idx, loop_perf[note_idx], to_screen);
            shapes.push(shape);
        }
    }
}

fn note_success_shape(
    beats_offset: f64,
    row: usize,
    acc: Accuracy,
    to_screen: RectTransform,
) -> Shape {
    let col = beats_offset as usize; // TODO: truncate, for now
    let rect = rect_for_col_row(col, row, to_screen);

    let bar_color = match acc {
        Accuracy::Early => ORANGE,
        Accuracy::Late => PURPLE,
        Accuracy::Correct => GREEN,
        Accuracy::Miss => RED,
        Accuracy::Unknown => GRAY,
    };
    let bar_color_32 = Color32::from_rgb(
        (bar_color.r * 256.) as u8,
        (bar_color.g * 256.) as u8,
        (bar_color.b * 256.) as u8,
    );

    egui::Shape::rect_filled(rect, egui::Rounding::default(), bar_color_32)
}

fn gold_mode(ui: &mut egui::Ui) {
    ui.add(egui::Label::new("**Gold Mode**"));

    // Simpler than chart..
    // 🔴
    // 🟠
    // 🟡
    // 🟢
    // ✅

    // convert scopes to summary

    // https://www.egui.rs/#demo check out FontBook to see supported black and white emoji
    // Can use image instead to get colors
    ui.add(egui::Label::new("✅🟢🟢🟢🟡🟠🟡🔴🔴"));

    // PLOT
    // let points = vec![[0., 0.7], [1., 0.5], [2., 0.3], [3., 0.1], [4., 0.0]];
    // let line = Line::new(points)
    //     .color(Color32::from_rgb(100, 200, 100))
    //     // .style(self.line_style)
    //     .name("gold_mode");

    // let plot = Plot::new("lines_demo")
    //     .legend(Legend::default())
    //     .show_axes(true)
    //     .show_grid(true);

    // plot.show(ui, |plot_ui| {
    //     plot_ui.line(line);
    // });
}

fn powered_by_egui_and_eframe(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.label("Powered by ");
        ui.hyperlink_to("egui", "https://github.com/emilk/egui");
        ui.label(" and ");
        ui.hyperlink_to(
            "eframe",
            "https://github.com/emilk/egui/tree/master/crates/eframe",
        );
        ui.label(".");
    });
}
