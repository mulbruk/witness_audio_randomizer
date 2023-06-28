use nwd::NwgUi;
use nwg::NativeUi;
use rand::{
  thread_rng,
  RngCore,
};
use std::{
  cell::RefCell,
  collections::hash_map::DefaultHasher,
  ffi::OsString,
  hash::Hasher,
  path::PathBuf,
};

use crate::core::{
  config::Config,
  witness_data,
  witness_data::DataStore,
};

use super::{
  create_backups::CreateBackupsDialogue,
  message_box::MessageBox,
  randomizer::RandomizerWindow,
  restore_backups::RestoreBackupsDialogue,
};

#[derive(Debug, Default)]
struct GuiState {
  witness_dir_okay: bool,
  logs_dir_okay: bool,

  dialogue_open: bool,
}

#[derive(Default, NwgUi)]
pub struct RandoGui {
  config: RefCell<Config>,
  state: RefCell<GuiState>,

  #[nwg_control(
    size: (800, 180),
    position: (640, 300),
    title: "Witness Audio Log Randomizer",
  )]
  #[nwg_events( OnInit: [RandoGui::on_window_load], OnWindowClose: [RandoGui::on_window_close] )]
  window: nwg::Window,

  #[nwg_control]
  #[nwg_events( OnNotice: [RandoGui::dialogue_closed] )]
  dialogue_notice: nwg::Notice,

  #[nwg_layout(parent: window, spacing: 4, min_size: [800, 180], max_size: [800, 180])]
  grid: nwg::GridLayout,

  // ---------------------------
  // Row 0

  #[nwg_control( text: "Witness directory:" )]
  #[nwg_layout_item(layout: grid, col: 0, row: 0, col_span: 4)]
  witness_dir_label: nwg::Label,

  #[nwg_control( text: "" )]
  #[nwg_layout_item(layout: grid, col: 4, row: 0, col_span: 15)]
  #[nwg_events( OnTextInput: [RandoGui::change_witness_dir_input] )]
  witness_dir_input: nwg::TextInput,

  #[nwg_control( text: "..." )]
  #[nwg_layout_item(layout: grid, col: 19, row: 0)]
  #[nwg_events( OnButtonClick: [RandoGui::click_witness_dir_picker])]
  witness_dir_button: nwg::Button,

  // ---------------------------
  // Row 1

  #[nwg_control( text: "Seed value:" )]
  #[nwg_layout_item(layout: grid, col: 0, row: 1, col_span: 4)]
  seed_label: nwg::Label,

  #[nwg_control( text: "" )]
  #[nwg_layout_item(layout: grid, col: 4, row: 1, col_span: 15)]
  #[nwg_events( OnTextInput: [RandoGui::change_seed_value_input] )]
  seed_input: nwg::TextInput,

  // ---------------------------
  // Row 2



  // ---------------------------
  // Row 3

  #[nwg_control( text: "Audio logs directory:" )]
  #[nwg_layout_item(layout: grid, col: 0, row: 3, col_span: 4)]
  logs_dir_label: nwg::Label,

  #[nwg_control( text: "" )]
  #[nwg_layout_item(layout: grid, col: 4, row: 3, col_span: 10)]
  #[nwg_events( OnTextInput: [RandoGui::change_logs_dir_input] )]
  logs_dir_input: nwg::TextInput,

  #[nwg_control( text: "..." )]
  #[nwg_layout_item(layout: grid, col: 14, row: 3)]
  #[nwg_events( OnButtonClick: [RandoGui::click_logs_dir_picker])]
  logs_dir_button: nwg::Button,

  #[nwg_control( text: "Restore data files", enabled: false )]
  #[nwg_layout_item(layout: grid, col: 16, row: 3, col_span: 4)]
  #[nwg_events( OnButtonClick: [RandoGui::click_restore_backups_button])]
  restore_backups_button: nwg::Button,

  // ---------------------------
  // Row 4

  #[nwg_control( text: "I'm feeling lucky", enabled: false )]
  #[nwg_layout_item(layout: grid, col: 7, row: 4, col_span: 4)]
  oops_all_secret_of_psalm_46_button: nwg::Button,

  #[nwg_control( text: "Randomize", enabled: false )]
  #[nwg_layout_item(layout: grid, col: 11, row: 4, col_span: 4)]
  #[nwg_events( OnButtonClick: [RandoGui::click_randomize_button])]
  randomize_button: nwg::Button,

  #[nwg_control( text: "Dump audio logs", enabled: false )]
  #[nwg_layout_item(layout: grid, col: 16, row: 4, col_span: 4)]
  #[nwg_events( OnButtonClick: [RandoGui::click_dump_logs_button])]
  dump_audio_button: nwg::Button,
}


impl RandoGui {
  pub fn start(config: Config) -> rando_gui_ui::RandoGuiUi {
    let config = RefCell::new(config);

    let app = RandoGui { config: config, ..Default::default() };
    RandoGui::build_ui(app).expect("Failed to build UI")
  }

  fn on_window_load(&self) {
    let cfg = self.config.borrow();
    
    self.witness_dir_input.set_text( cfg.witness_dir.to_str().unwrap() );
    self.logs_dir_input.set_text( cfg.logs_dir.to_str().unwrap() );

    let rng = &mut thread_rng();
    let seed = rng.next_u64();
    self.seed_input.set_text( &format!("{:X}", seed) );
  }

  fn on_window_close(&self) {
    let witness_dir = PathBuf::from( self.witness_dir_input.text() );
    let logs_dir = PathBuf::from( self.logs_dir_input.text() );
    let config = Config { witness_dir, logs_dir };

    let _ = config.save();

    nwg::stop_thread_dispatch()
  }

  fn dialogue_opened(&self) {
    {
      let mut state = self.state.borrow_mut();
      state.dialogue_open = true;
    }
    
    self.update_gui_state();
  }

  fn dialogue_closed(&self) {
    {
      let mut state = self.state.borrow_mut();
      state.dialogue_open = false;
    }
    
    self.update_gui_state();
  }

  fn change_witness_dir_input(&self) {
    let dir = PathBuf::from( self.witness_dir_input.text() );
    let okay;

    {
      let mut state = self.state.borrow_mut();
      okay = witness_data::witness_dir_is_okay(&dir);
      state.witness_dir_okay = okay;
    }

    if okay {
      let witness_dir = dir;

      if witness_data::data_needs_unpacking(&witness_dir) ||
         witness_data::data_needs_backing_up(&witness_dir) ||
         witness_data::subtitles_need_backing_up(&witness_dir) {
        self.dialogue_opened();
        CreateBackupsDialogue::show(&witness_dir, self.dialogue_notice.sender());
      }
    }
    
    self.update_gui_state();
  }

  fn click_witness_dir_picker(&self) {
    let current_value = self.witness_dir_input.text();
    let current_path = PathBuf::from(&current_value);

    let initial_dir = if current_path.is_dir() {
      Some(current_value.as_str())
    } else {
      None
    };

    self.fill_input_from_dir_picker(&self.witness_dir_input, initial_dir);
  }

  fn change_seed_value_input(&self) {}

  fn click_logs_dir_picker(&self) {
    let current_value = self.witness_dir_input.text();
    let current_path = PathBuf::from(&current_value);

    let initial_dir = if current_path.is_dir() {
      Some(current_value.as_str())
    } else {
      None
    };

    self.fill_input_from_dir_picker(&self.logs_dir_input, initial_dir);
  }

  fn change_logs_dir_input(&self) {
    let dir = PathBuf::from( self.logs_dir_input.text() );

    {
      let mut state = self.state.borrow_mut();
      state.logs_dir_okay = dir.is_dir();
    }

    self.update_gui_state();
  }

  fn click_dump_logs_button(&self) {
    let config = self.config.borrow();

    let dest_dir = if let Some(path) = self.show_directory_picker(None) {
      PathBuf::from(path)
    } else {
      return
    };

    self.dialogue_opened();

    let logs = DataStore::get_logs();
    let subs = match witness_data::load_subtitles(&config.witness_dir) {
      Ok(val) => val,
      Err(err) => {
        log::error!("Error reading subtitles file: {:?}", err);
        MessageBox::show("Could not dump subtitles file. See `application.log` for details.", self.dialogue_notice.sender());
        return
      }
    };

    match witness_data::dump_logs(&config.witness_dir, &dest_dir, &logs, &subs) {
      Ok(_) => {},
      Err(err) => {
        log::error!("Error dumping logs: {:?}", err);
        MessageBox::show("Could not dump audio files. See `application.log` for details.", self.dialogue_notice.sender());
        return
      }
    };

    MessageBox::show(&format!("Audio logs dumped to {:?}.", dest_dir), self.dialogue_notice.sender());
  }

  fn click_restore_backups_button(&self) {
    let config = self.config.borrow();

    self.dialogue_opened();
    RestoreBackupsDialogue::show(&config.witness_dir, self.dialogue_notice.sender());
  }

  fn click_randomize_button(&self) {
    let witness_dir = PathBuf::from( self.witness_dir_input.text() );
    let source_dir = PathBuf::from( self.logs_dir_input.text() );
    
    let mut hasher = DefaultHasher::new();
    let seed_string = self.seed_input.text();
    let seed_bytes = seed_string.as_bytes();
    hasher.write(&seed_bytes);
    let seed = hasher.finish();

    RandomizerWindow::show(&source_dir, &witness_dir, seed, self.dialogue_notice.sender());

    self.dialogue_opened();
  }

  fn fill_input_from_dir_picker(&self, text_box: &nwg::TextInput, initial_dir: Option<&str>) {
    if let Some(dir) = self.show_directory_picker(initial_dir) {
      if let Some(str) = dir.to_str() {
        text_box.set_text(str);
      } else {
        // TODO insert logging statement
      }
    }
  }

  fn update_gui_state(&self) {
    let state = self.state.borrow();

    self.witness_dir_button.set_enabled(
      !state.dialogue_open
    );

    self.logs_dir_button.set_enabled(
      !state.dialogue_open
    );

    self.randomize_button.set_enabled(
      !state.dialogue_open && state.witness_dir_okay && state.logs_dir_okay
    );

    self.oops_all_secret_of_psalm_46_button.set_enabled(
      !state.dialogue_open && state.witness_dir_okay
    );
    
    self.restore_backups_button.set_enabled(
      !state.dialogue_open && state.witness_dir_okay
    );
    
    self.dump_audio_button.set_enabled(
      !state.dialogue_open && state.witness_dir_okay
    );
  }

  fn show_directory_picker(&self, initial_dir: Option<&str>) -> Option<OsString> {
    let mut directory_picker: nwg::FileDialog = nwg::FileDialog::default();

    let builder = nwg::FileDialog::builder()
      .title("Select Folder")
      .action(nwg::FileDialogAction::OpenDirectory);
    let builder_result = {
      if let Some(dir) = initial_dir {
        builder
          .default_folder(dir)
          .build(&mut directory_picker)
      } else {
        builder.build(&mut directory_picker)
      }
    };
      
    if builder_result.is_err() {
      log::error!("Failed to create directory picker: {:?}", builder_result);
      return None;
    }

    if directory_picker.run(Some(&self.window)) {
      match directory_picker.get_selected_item() {
        Ok(result) => {
          Some(result)
        },
        Err(err) => {
          log::error!("Directory picker failure: {:?}", err);
          None
        },
      }
    } else {
      None
    }
  }
}
