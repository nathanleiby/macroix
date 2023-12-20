mod audio;
mod config;
mod consts;
mod input;
mod ui;
mod voices;

use std::error::Error;

use crate::audio::*;
use crate::config::AppConfig;
use crate::input::*;
use crate::ui::*;
use crate::voices::Voices;

use macroquad::prelude::*;

fn window_conf() -> Conf {
    Conf {
        window_title: "Macroix".to_owned(),
        // fullscreen: true,
        window_width: 1280,
        window_height: 720,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() -> Result<(), Box<dyn Error>> {
    let conf = AppConfig::new()?;
    dbg!(&conf);

    // Setup global game state
    let l = "res/loops/bulls_on_parade_1b.json";
    // let l = "res/loops/samba.json";
    // let l = "https://gist.githubusercontent.com/nathanleiby/6c35912e4c5d46351853f3225802a094/raw/7099fbdc934c7e67f1520276c319903ffbb8f5fb/bulls_on_parade_1.json";
    let mut voices = Voices::new_from_file(l).await?;

    let mut audio = Audio::new(&conf);

    let ui = UI::new(); // Consider passing in audio and voices here?

    loop {
        audio.schedule(&voices).await?;
        handle_user_input(&mut voices, &mut audio)?;
        ui.render(&voices, &audio);

        // wait for next frame from game engine
        next_frame().await
    }
}
