use nwd::NwgUi;
use nwg::NativeUi;
use std::{
  cell::RefCell,
  path::{Path, PathBuf},
  thread,
};

use crate::core::{
  witness_data,
};

#[derive(Debug, Default)]
struct CreateBackupsParams {
  witness_dir: PathBuf,
}

#[derive(Default, NwgUi)]
pub struct CreateBackupsDialogue {
  params: RefCell<CreateBackupsParams>,

  #[nwg_control(size: (640, 200), position: (650, 300), title: "", flags: "VISIBLE")]
  #[nwg_events( OnInit: [CreateBackupsDialogue::run], OnWindowClose: [CreateBackupsDialogue::close] )]
  window: nwg::Window,

  #[nwg_control(size: (600, 32), position: (20, 20), step: 0, range: 0..10 )]
  progress_bar: nwg::ProgressBar,

  #[nwg_control(size: (600, 32), position: (20, 56), text: "", h_align: HTextAlign::Center )]
  progress_text: nwg::Label,

  #[nwg_control(size: (64, 32), position: (268, 116), enabled: false, text: "Close")]
  #[nwg_events( OnButtonClick: [CreateBackupsDialogue::close])]
  close_button: nwg::Button,
}

impl CreateBackupsDialogue {
  pub fn show(witness_dir: &Path, sender: nwg::NoticeSender) {
    let dir = witness_dir.to_owned();

    thread::spawn(move || {
      let params = CreateBackupsParams { witness_dir: dir };
      let params = RefCell::new(params);
      let dialogue = CreateBackupsDialogue { params, ..Default::default() };
      let _ui = CreateBackupsDialogue::build_ui(dialogue).expect("Failed to build UI");
      nwg::dispatch_thread_events();
      
      // Notify the main thread that the dialogue completed
      sender.notice();
    });
  }

  fn run(&self) {
    let params = self.params.borrow();

    self.progress_bar.set_range(0..3);
    self.progress_bar.set_step(1);

    if witness_data::data_needs_unpacking(&params.witness_dir) {
      self.progress_text.set_text("Unpacking data files");
      match witness_data::unpack_witness_data(&params.witness_dir) {
        Ok(()) => {},
        Err(err) => {
          log::error!("Error unpacking data files: {:?}", err);
        },
      };
      self.progress_bar.advance();
    }

    if witness_data::data_needs_backing_up(&params.witness_dir) {
      self.progress_text.set_text("Backing up data files");
      match witness_data::create_audio_backup(&params.witness_dir) {
        Ok(()) => {},
        Err(err) => {
          log::error!("Error backing up data files: {:?}", err);
        },
      };
      self.progress_bar.advance();
    }

    if witness_data::subtitles_need_backing_up(&params.witness_dir) {
      self.progress_text.set_text("Backing up subtitles file");
      match witness_data::create_subtitles_backup(&params.witness_dir) {
        Ok(()) => {},
        Err(err) => {
          log::error!("Error backup up subtitles file: {:?}", err);
        },
      };
      self.progress_bar.advance();
    }

    let steps = 3 - self.progress_bar.step();
    self.progress_bar.advance_delta(steps);

    self.progress_text.set_text("Data files backed up and unpacked");
    
    self.close_button.set_enabled(true);
  }

  fn close(&self) {
    nwg::stop_thread_dispatch();
  }
}
