use nwd::NwgUi;
use nwg::NativeUi;
use std::{
  cell::RefCell,
  collections::HashMap,
  path::{Path, PathBuf},
  thread,
};

use crate::core::{
  witness_data,
  witness_data::{AudioLog, DataStore, SoundDestination, SoundInsertion, SoundInsertionMap, SubsInsertionMap, Subtitle},
};

fn secret_of_psalm_46_ogg_path(witness_dir: &Path) -> PathBuf { witness_dir.join(r"data/videos/psalm46.ogg") }

#[derive(Debug, Default)]
struct FeelingLuckyWindowParams {
  witness_dir: PathBuf,
}

#[derive(Default, NwgUi)]

pub struct FeelingLuckyWindow {
  params: RefCell<FeelingLuckyWindowParams>,

  #[nwg_control(size: (640, 200), position: (650, 300), title: "", flags: "VISIBLE")]
  #[nwg_events( OnInit: [FeelingLuckyWindow::run], OnWindowClose: [FeelingLuckyWindow::close] )]
  window: nwg::Window,

  #[nwg_control(size: (600, 32), position: (20, 20), step: 0, range: 0..10 )]
  progress_bar: nwg::ProgressBar,

  #[nwg_control(size: (600, 32), position: (20, 56), text: "", h_align: HTextAlign::Center )]
  progress_text: nwg::Label,

  #[nwg_control(size: (64, 32), position: (268, 116), enabled: false, text: "Close")]
  #[nwg_events( OnButtonClick: [FeelingLuckyWindow::close])]
  close_button: nwg::Button,
}

impl FeelingLuckyWindow {
  pub fn show(witness_dir: &Path, sender: nwg::NoticeSender) {
    let witness_dir = witness_dir.to_owned();

    thread::spawn(move || {
      let params = RefCell::new(
        FeelingLuckyWindowParams {witness_dir}
      );
      let dialogue = FeelingLuckyWindow { params, ..Default::default() };
      let _ui = FeelingLuckyWindow::build_ui(dialogue).expect("Failed to build UI");
      nwg::dispatch_thread_events();
      
      // Notify the main thread that the dialogue completed
      sender.notice();
    });
  }

  fn run(&self) {
    let params = self.params.borrow();

    let logs_data = DataStore::get_logs();
    let subs_data = match witness_data::load_subtitles(&params.witness_dir) {
      Ok(data) => data,
      Err(err) => {
        log::error!("Error loading subtitles file: {:?}", err);
        self.progress_text.set_text("Failure - see logs for more details");
        self.close_button.set_enabled(true);
        return;
      }
    };

    let secret_of_psalm_46_path = secret_of_psalm_46_ogg_path(&params.witness_dir);
    let secret_of_psalm_46_subs = subs_data.iter()
      .find(|Subtitle {key, val: _val}| key == "psalm46")
      .unwrap_or(&Subtitle {key: String::from(""), val: String::from("")})
      .to_owned();
    let pwd = std::env::current_dir().unwrap();
    let subs_file = pwd.join("psalm46.sub");
    
    let write_result = std::fs::write(&subs_file, secret_of_psalm_46_subs.val);
    if write_result.is_err() {
      log::error!("Error writing temporary subs file: {:?}", write_result);
      self.progress_text.set_text("Failure - see logs for more details");
      self.close_button.set_enabled(true);
      return;
    }

    let mut logs: SoundInsertionMap = HashMap::new();
    let mut subs: SubsInsertionMap = HashMap::new();

    let mut error_count = 0;

    for AudioLog {package, filename, subtitle} in logs_data {
      let dest_pkg = if let Some(path) = package {
        SoundDestination::Package(path)
      } else {
        SoundDestination::Root
      };

      let insertion = SoundInsertion { source_file: secret_of_psalm_46_path.clone(), dest_file: filename };

      if logs.contains_key(&dest_pkg) {
        let d: &mut Vec<SoundInsertion> = logs.get_mut(&dest_pkg).unwrap();
        d.push(insertion); 
      } else {
        logs.insert(dest_pkg, vec![insertion]);
      }

      subs.insert(subtitle, Some(subs_file.clone()));
    }

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
        error_count += 1;
      }

      self.progress_bar.advance();
    }

    self.progress_text.set_text("Updating subtitles");
    match witness_data::insert_subtitles(&params.witness_dir, subs_data, subs) {
      Ok(()) => {},
      Err(err) => {
        log::error!("Subtitles insertion failed: {:?}", err);
        error_count += 1;
      },
    };
    self.progress_bar.advance();

    let _ = std::fs::remove_file(&subs_file);
    
    if error_count == 0 {
      self.progress_text.set_text("Finished successfully");
    } else {
      self.progress_text.set_text(&format!("Finished with {} error(s)", error_count));
    }
    self.close_button.set_enabled(true);
  }

  fn close(&self) {
    nwg::stop_thread_dispatch();
  }
}
