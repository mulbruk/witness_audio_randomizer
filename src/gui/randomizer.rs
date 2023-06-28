use nwd::NwgUi;
use nwg::NativeUi;
use std::{
  cell::RefCell,
  path::{Path, PathBuf},
  thread,
};

use crate::core::{
  randomizer,
  witness_data,
  witness_data::SoundDestination,
};
// ---------------------------------------------------------------------------------------------------

#[derive(Debug, Default)]
struct RandomizerWindowParams {
  seed: u64,
  
  source_dir: PathBuf,
  witness_dir: PathBuf,
}

#[derive(Default, NwgUi)]

pub struct RandomizerWindow {
  params: RefCell<RandomizerWindowParams>,

  #[nwg_control(size: (640, 200), position: (650, 300), title: "", flags: "VISIBLE")]
  #[nwg_events( OnInit: [RandomizerWindow::run], OnWindowClose: [RandomizerWindow::close] )]
  window: nwg::Window,

  #[nwg_control(size: (600, 32), position: (20, 20), step: 0, range: 0..10 )]
  progress_bar: nwg::ProgressBar,

  #[nwg_control(size: (600, 32), position: (20, 56), text: "", h_align: HTextAlign::Center )]
  progress_text: nwg::Label,

  #[nwg_control(size: (64, 32), position: (268, 116), enabled: false, text: "Close")]
  #[nwg_events( OnButtonClick: [RandomizerWindow::close])]
  close_button: nwg::Button,
}

impl RandomizerWindow {
  pub fn show(source_dir: &Path, witness_dir: &Path, seed: u64, sender: nwg::NoticeSender) {
    let source_dir = source_dir.to_owned();
    let witness_dir = witness_dir.to_owned();

    thread::spawn(move || {
      let params = RefCell::new(
        RandomizerWindowParams {source_dir, witness_dir, seed}
      );
      let dialogue = RandomizerWindow { params, ..Default::default() };
      let _ui = RandomizerWindow::build_ui(dialogue).expect("Failed to build UI");
      nwg::dispatch_thread_events();
      
      // Notify the main thread that the dialogue completed
      sender.notice();
    });
  }

  fn run(&self) {
    let params = self.params.borrow();

    let subs_data = match witness_data::load_subtitles(&params.witness_dir) {
      Ok(data) => data,
      Err(err) => {
        log::error!("Error loading subtitles file: {:?}", err);
        // TODO maybe show a better indication of this failure
        return;
      }
    };
    
    let (logs, subs) = randomizer::randomize(params.seed, &params.source_dir);

    let progress_bar_range = 0..((logs.len() + 1) as u32);
    println!("Range: {:?}", progress_bar_range);
    self.progress_bar.set_range(progress_bar_range);
    self.progress_bar.set_step(1);
    self.progress_text.set_text("Randomizing");

    for (key, vals) in logs {
      let dest = match &key {
        SoundDestination::Package(pkg) => pkg.to_string_lossy().to_string(),
        SoundDestination::Root => String::from("data-pc/"),
      };
      self.progress_text.set_text( &format!("Randomizing logs in {}", dest) );

      let result = witness_data::insert_sound_files(
        vals,
        key,
        &params.witness_dir
      );

      if result.is_err() {
        log::error!("Sound insertion failed: {:?}", result);
      }

      self.progress_bar.advance();
    }

    self.progress_text.set_text("Updating subtitles");
    // Deal with subtitles
    match witness_data::insert_subtitles(&params.witness_dir, subs_data, subs) {
      Ok(()) => {},
      Err(err) => {} // TODO handle failure 
    };
    self.progress_bar.advance();
    
    self.progress_text.set_text("Complete");
    self.close_button.set_enabled(true);
  }

  fn close(&self) {
    nwg::stop_thread_dispatch();
  }
}
