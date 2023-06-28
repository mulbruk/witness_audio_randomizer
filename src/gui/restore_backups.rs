use nwd::NwgUi;
use nwg::NativeUi;
use std::{
  cell::RefCell,
  path::{Path, PathBuf},
  thread,
};

use crate::core::witness_data;

#[derive(Debug, Default)]
struct RestoreBackupsParams {
  witness_dir: PathBuf,
}

#[derive(Default, NwgUi)]

pub struct RestoreBackupsDialogue {
  params: RefCell<RestoreBackupsParams>,

  #[nwg_control(size: (640, 200), position: (650, 300), title: "", flags: "VISIBLE")]
  #[nwg_events( OnInit: [RestoreBackupsDialogue::run], OnWindowClose: [RestoreBackupsDialogue::close] )]
  window: nwg::Window,

  #[nwg_control(size: (600, 32), position: (20, 20), step: 0, range: 0..10 )]
  progress_bar: nwg::ProgressBar,

  #[nwg_control(size: (600, 32), position: (20, 56), text: "", h_align: HTextAlign::Center )]
  progress_text: nwg::Label,

  #[nwg_control(size: (64, 32), position: (268, 116), enabled: false, text: "Close")]
  #[nwg_events( OnButtonClick: [RestoreBackupsDialogue::close])]
  close_button: nwg::Button,
}

impl RestoreBackupsDialogue {
  pub fn show(witness_dir: &Path, sender: nwg::NoticeSender) {
    let dir = witness_dir.to_owned();

    thread::spawn(move || {
      let params = RestoreBackupsParams { witness_dir: dir };
      let params = RefCell::new(params);
      let dialogue = RestoreBackupsDialogue { params, ..Default::default() };
      let _ui = RestoreBackupsDialogue::build_ui(dialogue).expect("Failed to build UI");
      nwg::dispatch_thread_events();
      
      // Notify the main thread that the dialogue completed
      sender.notice();
    });
  }

  fn run(&self) {
    let params = self.params.borrow();
    let mut success: bool = true;

    self.progress_text.set_text("Restoring subtitles");
    match witness_data::restore_subtitles_backup(&params.witness_dir) {
      Ok(_) => {},
      Err(err) => {
        log::error!("Failure restoring subtitle backups: {:?}", err);
        success = false;
      }
    };
    self.progress_bar.advance_delta(2);

    self.progress_text.set_text("Restoring audio files");
    match witness_data::restore_audio_backup(&params.witness_dir) {
      Ok(_) => {},
      Err(err) => {
        log::error!("Failure restoring audio file backups: {:?}", err);
        success = false;
      }
    };
    self.progress_bar.advance_delta(8);

    if success {
      self.progress_text.set_text("Data files restored successfully");
    } else {
      self.progress_text.set_text("An error occurred while restoring data files, see app.log for more details");
    }
    self.close_button.set_enabled(true);
  }

  fn close(&self) {
    nwg::stop_thread_dispatch();
  }
}
