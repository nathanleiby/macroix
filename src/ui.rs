use crate::{audio::Audio, consts::*, Voices};

use macroquad::{audio, prelude::*};

pub struct UI {}

impl UI {
    pub fn new() -> Self {
        Self {}
    }

    pub fn render(self: &Self, voices: &Voices, audio: &Audio) {
        let current_beat = audio.current_beat();
        let audio_latency = audio.get_configured_audio_latency_seconds();
        let bpm = audio.get_bpm();

        clear_background(LIGHTGRAY);
        draw_beat_grid(voices);
        draw_user_hits(&audio.user_hits);
        draw_position_line(current_beat + audio_latency);
        draw_status(bpm, current_beat / 2., audio_latency);

        draw_pulse_beat(current_beat + audio_latency);
    }
}

fn draw_status(bpm: f64, current_beat: f64, audio_latency: f64) {
    draw_text(
        format!("BPM: {bpm}").as_str(),
        (GRID_LEFT_X) as f32,
        20.0,
        30.0,
        DARKGRAY,
    );
    draw_text(
        format!("Current Beat: {:.1}", current_beat).as_str(),
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

fn draw_beat_grid(voices: &Voices) {
    let closed_hihat_notes = &voices.closed_hihat;
    let snare_notes = &voices.snare;
    let kick_notes = &voices.kick;
    let open_hihat_notes = &voices.open_hihat;

    // Labels in top-left of grid
    for (idx, name) in ["Hihat", "Snare", "Kick", "Open hi-hat"].iter().enumerate() {
        draw_text(
            name,
            20.0,
            (GRID_TOP_Y + ROW_HEIGHT * (idx as f64 + 0.5)) as f32,
            20.0,
            DARKGRAY,
        );
    }

    let start_x = GRID_LEFT_X;
    let start_y = GRID_TOP_Y;
    for i in 0..=(NUM_ROWS_IN_GRID as usize) {
        let y = start_y + i as f64 * ROW_HEIGHT;
        draw_line_f64(start_x, y, start_x + GRID_WIDTH, y, 4.0, BLACK);
    }

    // draw vertical lines every 4 beats
    for i in 0..=(BEATS_PER_LOOP as i32) {
        let x = start_x + i as f64 * BEAT_WIDTH_PX;
        draw_line_f64(
            x,
            start_y,
            x,
            start_y + ROW_HEIGHT * NUM_ROWS_IN_GRID,
            4.0,
            BLACK,
        );
    }

    for note in closed_hihat_notes.iter() {
        draw_note(*note, 0);
    }

    for note in snare_notes.iter() {
        draw_note(*note, 1);
    }

    // same kick notes but with a lead up to each note
    for note in kick_notes.iter() {
        draw_note(*note, 2);
    }

    // same kick notes but with a lead up to each note
    for note in open_hihat_notes.iter() {
        draw_note(*note, 3);
    }
}

fn draw_user_hits(voices: &Voices) {
    let closed_hihat_notes = &voices.closed_hihat;
    let snare_notes = &voices.snare;
    let kick_notes = &voices.kick;
    let open_hihat_notes = &voices.open_hihat;

    // // Labels in top-left of grid
    // for (idx, name) in ["Hihat", "Snare", "Kick", "Open hi-hat"].iter().enumerate() {
    //     draw_text(
    //         name,
    //         20.0,
    //         (GRID_TOP_Y + ROW_HEIGHT * (idx as f64 + 0.5)) as f32,
    //         20.0,
    //         DARKGRAY,
    //     );
    // }

    // let start_x = GRID_LEFT_X;
    // let start_y = GRID_TOP_Y;
    // for i in 0..=(NUM_ROWS_IN_GRID as usize) {
    //     let y = start_y + i as f64 * ROW_HEIGHT;
    //     draw_line_f64(start_x, y, start_x + GRID_WIDTH, y, 4.0, BLACK);
    // }

    // // draw vertical lines every 4 beats
    // for i in 0..=(BEATS_PER_LOOP as i32) {
    //     let x = start_x + i as f64 * BEAT_WIDTH_PX;
    //     draw_line_f64(
    //         x,
    //         start_y,
    //         x,
    //         start_y + ROW_HEIGHT * NUM_ROWS_IN_GRID,
    //         4.0,
    //         BLACK,
    //     );
    // }

    for note in closed_hihat_notes.iter() {
        draw_user_hit(*note, 0);
    }

    for note in snare_notes.iter() {
        draw_user_hit(*note, 1);
    }

    // same kick notes but with a lead up to each note
    for note in kick_notes.iter() {
        draw_user_hit(*note, 2);
    }

    // same kick notes but with a lead up to each note
    for note in open_hihat_notes.iter() {
        draw_user_hit(*note, 3);
    }
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
        Color {
            r: 1.0,
            g: 0.63 + row as f32 * 0.1,
            b: 0.0 + row as f32 * 0.1,
            a: 1.0,
        },
    );
}

fn draw_user_hit(beats_offset: f64, row: usize) {
    // let beat_duration = 1 as f64;
    let beat_duration = 0.1 as f64; // make it thin for easier overlap, for now
    let x = GRID_LEFT_X + beats_offset * BEAT_WIDTH_PX;
    let y = GRID_TOP_Y + row as f64 * ROW_HEIGHT;
    draw_rectangle_f64(
        x + BEAT_PADDING / 2.,
        y + BEAT_PADDING / 2.,
        BEAT_WIDTH_PX * beat_duration - BEAT_PADDING,
        BEAT_WIDTH_PX - BEAT_PADDING,
        Color {
            r: 0.0,
            g: 0.63 + row as f32 * 0.1,
            b: 0.0 + row as f32 * 0.1,
            a: 1.0,
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

    let r = 100.;
    if dist < 0.05 {
        let scale = (0.1 - dist) * 10.;
        draw_circle(
            screen_width() / 2.,
            screen_height() / 2. + 100.,
            r * scale as f32,
            RED,
        );
    }
}
