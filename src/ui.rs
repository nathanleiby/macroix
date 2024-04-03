use crate::{
    audio::Audio,
    consts::*,
    score::{compute_accuracy, Accuracy},
    voices::{Instrument, Loop},
    UserHit, Voices,
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
    ) {
        let current_beat = audio.current_beat();

        let audio_latency = audio.get_configured_audio_latency_seconds();
        let bpm = audio.get_bpm();

        clear_background(BACKGROUND_COLOR);
        draw_beat_grid(voices);
        draw_user_hits(&audio.user_hits, &voices, audio_latency);
        draw_position_line(current_beat + audio_latency);

        // TODO: render current loop considering audio latency
        draw_status(bpm, current_beat / 2., audio.current_loop(), audio_latency);

        let last_loop_hits = get_last_loop_hits(audio);
        draw_last_loop_summary(&last_loop_hits, &voices, audio_latency);

        // TODO: toggle this on and off with a key for 'calibration' mode
        draw_pulse_beat(current_beat + audio_latency);

        draw_loop_choices(voices, audio, &loops);
    }
}

fn get_last_loop_hits(audio: &mut Audio) -> Vec<UserHit> {
    let last_loop_hits: Vec<UserHit> = audio
        .user_hits
        .iter()
        .filter(|hit| {
            let loop_num_forbeat = (hit.clock_tick / BEATS_PER_LOOP) as i32;
            loop_num_forbeat == audio.current_loop() - 1
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

fn get_user_hit_timings_by_instrument(
    user_hits: &Vec<UserHit>,
    instrument: Instrument,
) -> Vec<f64> {
    user_hits
        .iter()
        .filter(|hit| hit.instrument == instrument)
        .map(|hit| hit.beat())
        .collect::<Vec<f64>>()
}

fn draw_user_hits(user_hits: &Vec<UserHit>, desired_hits: &Voices, audio_latency: f64) {
    // filter user hits to just closed hihat
    let closed_hihat_notes = get_user_hit_timings_by_instrument(user_hits, Instrument::ClosedHihat);
    let snare_notes = get_user_hit_timings_by_instrument(user_hits, Instrument::Snare);
    let kick_notes = get_user_hit_timings_by_instrument(user_hits, Instrument::Kick);
    let open_hihat_notes = get_user_hit_timings_by_instrument(user_hits, Instrument::OpenHihat);

    for note in closed_hihat_notes.iter() {
        draw_user_hit(*note, 0, audio_latency, &desired_hits.closed_hihat);
    }

    for note in snare_notes.iter() {
        draw_user_hit(*note, 1, audio_latency, &desired_hits.snare);
    }

    // same kick notes but with a lead up to each note
    for note in kick_notes.iter() {
        draw_user_hit(*note, 2, audio_latency, &desired_hits.kick);
    }

    // same kick notes but with a lead up to each note
    for note in open_hihat_notes.iter() {
        draw_user_hit(*note, 3, audio_latency, &desired_hits.open_hihat);
    }
}

fn draw_last_loop_summary(user_hits: &Vec<UserHit>, desired_hits: &Voices, audio_latency: f64) {
    let mut total_score: usize = 0;
    let mut total_notes: usize = 0;

    let instruments = [
        Instrument::ClosedHihat,
        Instrument::Snare,
        Instrument::Kick,
        Instrument::OpenHihat,
    ];
    for (idx, instrument) in instruments.iter().enumerate() {
        // get accuracy of hihat
        let user_timings = get_user_hit_timings_by_instrument(user_hits, *instrument);
        let desired_timings = get_desired_timings_by_instrument(instrument, desired_hits);

        // compare that to desired hits for hihat
        let mut note_idx: i32 = 0;
        let mut num_correct: usize = 0;
        for note in user_timings.iter() {
            let (acc, _) = compute_accuracy(note + audio_latency, desired_timings);
            if acc == Accuracy::Correct {
                num_correct += 1;
            }
            note_idx += 1;
        }

        // Labels in top-left of grid
        draw_text(
            format!("{num_correct} / {:?}", desired_timings.len()).as_str(),
            (GRID_LEFT_X + GRID_WIDTH + 32.) as f32,
            (GRID_TOP_Y + ROW_HEIGHT * (idx as f64 + 0.5)) as f32,
            20.0,
            DARKGRAY,
        );

        total_score += num_correct;
        total_notes += desired_timings.len();
    }

    // Summary over all voices
    draw_text(
        format!("{total_score} / {:?}", total_notes).as_str(),
        (GRID_LEFT_X + GRID_WIDTH + 32.) as f32,
        (GRID_TOP_Y + ROW_HEIGHT * (instruments.len() as f64 + 0.5)) as f32,
        20.0,
        DARKGRAY,
    );

    // TODO: div by zero issue -> shows NaN
    let score_ratio = total_score as f64 / total_notes as f64;
    draw_circle(
        (GRID_LEFT_X + GRID_WIDTH + 32.) as f32,
        (GRID_TOP_Y + ROW_HEIGHT * ((instruments.len() + 1) as f64 + 0.5)) as f32,
        64.,
        Color {
            r: 1. - score_ratio as f32,
            g: score_ratio as f32,
            b: 0.,
            a: 1.,
        },
    );
    draw_text(
        format!("{:.1}%", score_ratio * 100.).as_str(),
        (GRID_LEFT_X + GRID_WIDTH - 32. + 8.) as f32,
        (GRID_TOP_Y + ROW_HEIGHT * ((instruments.len() + 1) as f64 + 0.5) + 16.) as f32,
        64.,
        WHITE,
    );
}

fn get_desired_timings_by_instrument<'a>(
    instrument: &Instrument,
    desired_hits: &'a Voices,
) -> &'a Vec<f64> {
    let desired_timings = match instrument {
        Instrument::ClosedHihat => &desired_hits.closed_hihat,
        Instrument::Snare => &desired_hits.snare,
        Instrument::Kick => &desired_hits.kick,
        Instrument::OpenHihat => &desired_hits.open_hihat,
    };
    desired_timings
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

fn draw_user_hit(user_beat: f64, row: usize, audio_latency: f64, desired_hits: &Vec<f64>) {
    let user_beat_with_latency = user_beat + audio_latency;

    let (acc, is_next_loop) = compute_accuracy(user_beat_with_latency, desired_hits);

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
