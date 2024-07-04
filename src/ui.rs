/*
  Display the UI.

  The UI is built in Macroquad's UI system, which is a simple immediate mode GUI system.
  Mostly, we draw lines and boxes.
  However, we also make use of EGUI for some items like the "choosing a loop".
*/
use crate::{
    audio::Audio,
    consts::*,
    score::{
        compute_accuracy_of_single_hit, compute_last_loop_summary,
        compute_loop_performance_for_voice, get_desired_timings_by_instrument,
        get_user_hit_timings_by_instrument, Accuracy, MISS_MARGIN,
    },
    voices::{Instrument, Loop},
    GoldMode, UserHit, Voices,
};

use macroquad::{prelude::*, ui::*};

const LINE_COLOR: Color = DARKGRAY;

const NOTE_COLOR: Color = GRAY;

const BACKGROUND_COLOR: Color = Color {
    r: 0.99,
    g: 0.99,
    b: 0.99,
    a: 1.0,
};

const ALL_INSTRUMENTS: [Instrument; 4] = [
    Instrument::ClosedHihat,
    Instrument::Snare,
    Instrument::Kick,
    Instrument::OpenHihat,
];

pub struct UI {}

impl UI {
    pub fn new() -> Self {
        Self {}
    }

    pub fn render(
        self: &mut Self,

        // TODO: consider bundling the below into a Game struct or similar
        voices: &mut Voices,
        audio: &mut Audio,
        loops: &Vec<(String, Loop)>,
        gold_mode: &GoldMode,
    ) {
        let current_beat = audio.current_beat();

        let audio_latency = audio.get_configured_audio_latency_seconds();
        let bpm = audio.get_bpm();

        clear_background(BACKGROUND_COLOR);
        draw_beat_grid(voices);
        draw_user_hits(&audio.user_hits, &voices, audio_latency);
        let loop_last_completed_beat = current_beat - MISS_MARGIN;
        let current_loop_hits = get_hits_from_nth_loop(&audio.user_hits, audio.current_loop());
        draw_note_successes(
            &current_loop_hits,
            &voices,
            audio_latency,
            loop_last_completed_beat,
        );
        draw_position_line(current_beat + audio_latency);

        // TODO: render current loop considering audio latency
        draw_status(bpm, current_beat / 2., audio.current_loop(), audio_latency);

        for i in 1..=3 {
            let last_loop_hits = get_hits_from_nth_loop(&audio.user_hits, audio.current_loop() - i);
            draw_loop_summary(&last_loop_hits, &voices, audio_latency, i);
        }

        // TODO: toggle this on and off with a key for 'calibration' mode
        draw_pulse_beat(current_beat + audio_latency);

        draw_loop_choices(voices, audio, &loops);

        draw_gold_mode(gold_mode);
    }
}

pub fn get_hits_from_nth_loop(user_hits: &Vec<UserHit>, desired_loop_idx: i32) -> Vec<UserHit> {
    let last_loop_hits: Vec<UserHit> = user_hits
        .iter()
        .filter(|hit| {
            // include hits from just before start of loop (back to 0 - MISS), since those could be early or on-time hits
            let loop_num_for_hit = ((hit.clock_tick + MISS_MARGIN) / BEATS_PER_LOOP) as i32;
            loop_num_for_hit == desired_loop_idx
        })
        .map(|hit| hit.clone())
        .collect::<Vec<UserHit>>();
    last_loop_hits
}

fn draw_status(bpm: f64, current_beat: f64, current_loop: i32, audio_latency: f64) {
    draw_text(
        format!("BPM: {bpm}").as_str(),
        (GRID_LEFT_X) as f32,
        20.0,
        30.0,
        DARKGRAY,
    );
    draw_text(
        format!("Current Beat={:.1} (Loop={:?})", current_beat, current_loop).as_str(),
        (GRID_LEFT_X) as f32,
        40.0,
        30.0,
        DARKGRAY,
    );
    draw_text(
        format!("Calibrated Latency: {:.3} seconds", audio_latency).as_str(),
        (GRID_LEFT_X) as f32,
        60.0,
        30.0,
        DARKGRAY,
    );
}

fn draw_beat_grid(desired_hits: &Voices) {
    let start_x = GRID_LEFT_X;
    let start_y = GRID_TOP_Y;

    // draw vertical lines every beats
    for i in 0..=(BEATS_PER_LOOP as i32) {
        let x = start_x + i as f64 * BEAT_WIDTH_PX;
        draw_line_f64(
            x,
            start_y,
            x,
            start_y + ROW_HEIGHT * NUM_ROWS_IN_GRID,
            // if i % 4 == 0 { 6.0 } else { 4.0 },
            4.0,
            if i % 4 == 0 { BLACK } else { LINE_COLOR },
        );
    }

    for i in 0..=(NUM_ROWS_IN_GRID as usize) {
        let y = start_y + i as f64 * ROW_HEIGHT;
        draw_line_f64(start_x, y, start_x + GRID_WIDTH, y, 4.0, BLACK);
    }

    for (instrument_idx, instrument) in ALL_INSTRUMENTS.iter().enumerate() {
        let name = match *instrument {
            Instrument::ClosedHihat => "Hihat",
            Instrument::Snare => "Snare",
            Instrument::Kick => "Kick",
            Instrument::OpenHihat => "Open hi-hat",
            Instrument::Ride => "Ride",
        };

        // Labels in top-left of grid
        draw_text(
            name,
            20.0,
            (GRID_TOP_Y + ROW_HEIGHT * (instrument_idx as f64 + 0.5)) as f32,
            20.0,
            DARKGRAY,
        );

        let desired = get_desired_timings_by_instrument(instrument, desired_hits);
        for note in desired.iter() {
            draw_note(*note, instrument_idx);
        }
    }
}

fn draw_user_hits(user_hits: &Vec<UserHit>, desired_hits: &Voices, audio_latency: f64) {
    for (instrument_idx, instrument) in ALL_INSTRUMENTS.iter().enumerate() {
        let user_notes = get_user_hit_timings_by_instrument(user_hits, *instrument);
        let desired_notes = get_desired_timings_by_instrument(instrument, desired_hits);
        for note in user_notes.iter() {
            draw_user_hit(*note, instrument_idx, audio_latency, desired_notes);
        }
    }
}

fn draw_note_successes(
    user_hits: &Vec<UserHit>,
    desired_hits: &Voices,
    audio_latency: f64,
    loop_current_beat: f64,
) {
    for (instrument_idx, instrument) in ALL_INSTRUMENTS.iter().enumerate() {
        let actual = get_user_hit_timings_by_instrument(user_hits, *instrument);
        // add audio_latency to each note
        let actual_w_latency = actual
            .iter()
            .map(|note| note + audio_latency)
            .collect::<Vec<f64>>();

        let desired = get_desired_timings_by_instrument(instrument, &desired_hits);

        let loop_perf =
            compute_loop_performance_for_voice(&actual_w_latency, &desired, loop_current_beat);
        for (note_idx, note) in desired.iter().enumerate() {
            draw_note_success(*note, instrument_idx, loop_perf[note_idx]);
        }
    }
}

fn draw_loop_summary(
    user_hits: &Vec<UserHit>,
    desired_hits: &Voices,
    audio_latency: f64,
    nth_loop: i32,
) {
    let summary_data = compute_last_loop_summary(user_hits, desired_hits, audio_latency);

    if nth_loop == 1 {
        // Show summary to the right of each instrument row
        for (idx, instrument) in ALL_INSTRUMENTS.iter().enumerate() {
            let num_correct = match instrument {
                Instrument::ClosedHihat => summary_data.hihat.num_correct,
                Instrument::Snare => summary_data.snare.num_correct,
                Instrument::Kick => summary_data.kick.num_correct,
                Instrument::OpenHihat => summary_data.open_hihat.num_correct,
                Instrument::Ride => summary_data.ride.num_correct,
            };
            let num_notes = match instrument {
                Instrument::ClosedHihat => summary_data.hihat.num_notes,
                Instrument::Snare => summary_data.snare.num_notes,
                Instrument::Kick => summary_data.kick.num_notes,
                Instrument::OpenHihat => summary_data.open_hihat.num_notes,
                Instrument::Ride => summary_data.ride.num_notes,
            };

            draw_text(
                format!("{num_correct} / {:?}", num_notes).as_str(),
                (GRID_LEFT_X + GRID_WIDTH + 32.) as f32,
                (GRID_TOP_Y + ROW_HEIGHT * (idx as f64 + 0.5)) as f32,
                20.0,
                DARKGRAY,
            );
        }
    }

    //
    // Show overall summary for the nth loop
    //

    let totals = summary_data.total();
    let total_score = totals.num_correct;
    let total_notes = totals.num_notes;

    let x_base = (GRID_LEFT_X + GRID_WIDTH + 32. - (nth_loop as f64 * (128. + 16.))) as f32;
    let y_base = (GRID_TOP_Y + ROW_HEIGHT * ((ALL_INSTRUMENTS.len() + 2) as f64)) as f32;

    // Summary over all voices
    let circle_radius = 64.;
    // TODO: div by zero issue -> shows NaN
    let score_ratio = totals.ratio();
    draw_circle(
        x_base,
        y_base + ROW_HEIGHT as f32,
        circle_radius,
        Color {
            r: 1. - score_ratio as f32,
            g: score_ratio as f32,
            b: 0.,
            a: 1.,
        },
    );

    draw_text(
        format!("{:.0}%", score_ratio * 100.).as_str(),
        x_base - circle_radius + 8.,
        y_base + (ROW_HEIGHT * 1.25) as f32,
        64.,
        WHITE,
    );

    draw_text(
        format!("{total_score} / {:?}", total_notes).as_str(),
        x_base - 16.,
        y_base,
        20.0,
        DARKGRAY,
    );
}

fn draw_position_line(current_beat: f64) {
    let start_x = GRID_LEFT_X;
    let start_y = GRID_TOP_Y;

    // draw a vertical line at the current positonj
    let x = start_x + current_beat * BEAT_WIDTH_PX;
    draw_line_f64(x, start_y, x, start_y + ROW_HEIGHT * 5., 4.0, RED);
}

fn draw_note(beats_offset: f64, row: usize) {
    let beat_duration = 1 as f64;
    let x = GRID_LEFT_X + beats_offset * BEAT_WIDTH_PX;
    let y = GRID_TOP_Y + row as f64 * ROW_HEIGHT;
    draw_rectangle_f64(
        x + BEAT_PADDING / 2.,
        y + BEAT_PADDING / 2.,
        BEAT_WIDTH_PX * beat_duration - BEAT_PADDING,
        BEAT_WIDTH_PX - BEAT_PADDING,
        NOTE_COLOR,
    );
}

fn draw_note_success(beats_offset: f64, row: usize, acc: Accuracy) {
    let beat_duration = 1 as f64;
    let x = GRID_LEFT_X + beats_offset * BEAT_WIDTH_PX;
    let y = GRID_TOP_Y + row as f64 * ROW_HEIGHT;
    let mut color = match acc {
        Accuracy::Early => ORANGE,
        Accuracy::Late => PURPLE,
        Accuracy::Correct => GREEN,
        Accuracy::Miss => RED,
        Accuracy::Unknown => GRAY,
    };
    color.a = 0.5;

    draw_rectangle_f64(
        x + BEAT_PADDING / 2.,
        y + BEAT_PADDING / 2.,
        BEAT_WIDTH_PX * beat_duration - BEAT_PADDING,
        BEAT_WIDTH_PX - BEAT_PADDING,
        color,
    );
}

fn draw_user_hit(user_beat: f64, row: usize, audio_latency: f64, desired_hits: &Vec<f64>) {
    let user_beat_with_latency = user_beat + audio_latency;

    let (acc, is_next_loop) = compute_accuracy_of_single_hit(user_beat_with_latency, desired_hits);

    let beat_duration = 0.1 as f64; // make it thin for easier overlap, for now

    // with audio latency
    let x = if is_next_loop {
        GRID_LEFT_X + (user_beat_with_latency - BEATS_PER_LOOP) * BEAT_WIDTH_PX
    } else {
        GRID_LEFT_X + user_beat_with_latency * BEAT_WIDTH_PX
    };

    let y = GRID_TOP_Y + row as f64 * ROW_HEIGHT;

    draw_rectangle_f64(
        x + BEAT_PADDING / 2.,
        y + BEAT_PADDING / 2.,
        BEAT_WIDTH_PX * beat_duration - BEAT_PADDING,
        BEAT_WIDTH_PX - BEAT_PADDING,
        match acc {
            Accuracy::Early => ORANGE,
            Accuracy::Late => PURPLE,
            Accuracy::Correct => GREEN,
            Accuracy::Miss => RED,
            Accuracy::Unknown => GRAY,
        },
    );
}

fn draw_rectangle_f64(x: f64, y: f64, width: f64, height: f64, color: Color) {
    draw_rectangle(x as f32, y as f32, width as f32, height as f32, color);
}

fn draw_line_f64(x1: f64, y1: f64, x2: f64, y2: f64, thickness: f32, color: Color) {
    draw_line(x1 as f32, y1 as f32, x2 as f32, y2 as f32, thickness, color);
}

fn draw_pulse_beat(current_beat: f64) {
    // every other beat
    if current_beat.floor() % 2. == 0. {
        return;
    }

    // get the distance from the current beat center
    let dist = (1. - current_beat % 1.).abs();
    if dist < 0.05 {
        draw_circle(screen_width() / 2., screen_height() / 2. + 100., 100., RED);
    }
}

const UI_TOP_LEFT: Vec2 = vec2(100., 400.);

fn draw_loop_choices<'a, 'b: 'a>(
    voices: &'a mut Voices,
    audio: &'a mut Audio,
    voices_options: &'b Vec<(String, Loop)>,
) {
    widgets::Window::new(hash!(), UI_TOP_LEFT, vec2(320., 200.))
        .label("Loops")
        .titlebar(true)
        .ui(&mut *root_ui(), |ui| {
            voices_options.iter().for_each(|(name, new_loop)| {
                if ui.button(None, format!("{:?} ({:?})", name.as_str(), new_loop.bpm)) {
                    *voices = new_loop.voices.clone();
                    audio.set_bpm(new_loop.bpm as f64);
                    log::info!("Switched to {:?}", name);
                }
            });
        });
}

fn draw_gold_mode(gold_mode: &GoldMode) {
    draw_text(
        format!(
            "Gold Mode = {} (was gold = {})",
            gold_mode.correct_takes, gold_mode.was_gold
        )
        .as_str(),
        420.,
        360.,
        32.,
        BLACK,
    )
}
