use nwd::NwgUi;
use nwg::NativeUi;
use std::{
  cell::RefCell,
  ffi::{OsStr, OsString},
  path::PathBuf,
};

use crate::core::{
  config::Config,
  witness_data,
};

use super::{
  create_backups::CreateBackupsDialogue,
  message_box::MessageBox,
};

#[derive(Debug, Default)]
struct GuiState {
  witness_dir_okay: bool,
  test_ogg_okay: bool,

  dialogue_open: bool,
}

#[derive(Default, NwgUi)]
pub struct TestToolGui {
  config: RefCell<Config>,
  state: RefCell<GuiState>,

  #[nwg_control(
    size: (800, 115),
    position: (640, 300),
    title: "Witness Audio Log Test Inserter",
  )]
  #[nwg_events( OnInit: [TestToolGui::on_window_load], OnWindowClose: [TestToolGui::on_window_close] )]
  window: nwg::Window,

  #[nwg_control]
  #[nwg_events( OnNotice: [TestToolGui::dialogue_closed] )]
  dialogue_notice: nwg::Notice,

  #[nwg_layout(parent: window, spacing: 4, min_size: [800, 115], max_size: [800, 115])]
  grid: nwg::GridLayout,

  // ---------------------------
  // Row 0

  #[nwg_control( text: "Witness directory:" )]
  #[nwg_layout_item(layout: grid, col: 0, row: 0, col_span: 4)]
  witness_dir_label: nwg::Label,

  #[nwg_control( text: "" )]
  #[nwg_layout_item(layout: grid, col: 4, row: 0, col_span: 15)]
  #[nwg_events( OnTextInput: [TestToolGui::change_witness_dir_input] )]
  witness_dir_input: nwg::TextInput,

  #[nwg_control( text: "..." )]
  #[nwg_layout_item(layout: grid, col: 19, row: 0)]
  #[nwg_events( OnButtonClick: [TestToolGui::click_witness_dir_picker])]
  witness_dir_button: nwg::Button,

  // ---------------------------
  // Row 1

  #[nwg_control( text: "Test file path:" )]
  #[nwg_layout_item(layout: grid, col: 0, row: 1, col_span: 4)]
  logs_dir_label: nwg::Label,

  #[nwg_control( text: "" )]
  #[nwg_layout_item(layout: grid, col: 4, row: 1, col_span: 15)]
  #[nwg_events( OnTextInput: [TestToolGui::change_test_file_input] )]
  test_file_input: nwg::TextInput,

  #[nwg_control( text: "..." )]
  #[nwg_layout_item(layout: grid, col: 19, row: 1)]
  #[nwg_events( OnButtonClick: [TestToolGui::click_test_file_picker])]
  logs_dir_button: nwg::Button,

  // ---------------------------
  // Row 2

  #[nwg_control( text: "Insert log", enabled: false )]
  #[nwg_layout_item(layout: grid, col: 16, row: 2, col_span: 4)]
  #[nwg_events( OnButtonClick: [TestToolGui::click_insert_button])]
  insert_button: nwg::Button,
}


impl TestToolGui {
  #[allow(dead_code)]
  pub fn start(config: Config) -> test_tool_gui_ui::TestToolGuiUi {
    let config = RefCell::new(config);

    let app = TestToolGui { config: config, ..Default::default() };
    TestToolGui::build_ui(app).expect("Failed to build UI")
  }

  fn on_window_load(&self) {
    let cfg = self.config.borrow();
    
    self.witness_dir_input.set_text( cfg.witness_dir.to_str().unwrap() );
  }

  fn on_window_close(&self) {
    let old_config = self.config.borrow();
    let witness_dir = PathBuf::from( self.witness_dir_input.text() );
    
    let config = Config { witness_dir, logs_dir: old_config.logs_dir.clone() };

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

  fn click_test_file_picker(&self) {
    let current_value = self.witness_dir_input.text();
    let current_path = PathBuf::from(&current_value);

    let initial_dir = if current_path.is_dir() {
      Some(current_value.as_str())
    } else {
      None
    };

    self.fill_input_from_file_picker(&self.test_file_input, initial_dir);
  }

  fn change_test_file_input(&self) {
    let path = PathBuf::from( self.test_file_input.text() );

    {
      let mut state = self.state.borrow_mut();
      state.test_ogg_okay = path.extension().and_then(OsStr::to_str) == Some("ogg") && path.exists();
    }

    self.update_gui_state();
  }

  fn click_insert_button(&self) {
    let witness_dir = PathBuf::from( self.witness_dir_input.text() );
    let source_path = PathBuf::from( self.test_file_input.text() );
    
    let result = witness_data::insert_on_mountaintop(&witness_dir, &source_path);
    if result.is_ok() {
      MessageBox::show("File inserted successfully", self.dialogue_notice.sender());
    } else {
      log::error!("Error inserting test file {:?}: {:?}", source_path, result);
      MessageBox::show("Error inserting file", self.dialogue_notice.sender());
    }

    self.dialogue_opened();
  }

  fn fill_input_from_dir_picker(&self, text_box: &nwg::TextInput, initial_dir: Option<&str>) {
    if let Some(dir) = self.show_directory_picker(initial_dir) {
      if let Some(str) = dir.to_str() {
        text_box.set_text(str);
      } else {
        // Nothing to do here
      }
    }
  }

  fn fill_input_from_file_picker(&self, text_box: &nwg::TextInput, initial_dir: Option<&str>) {
    if let Some(dir) = self.show_file_picker(initial_dir) {
      if let Some(str) = dir.to_str() {
        text_box.set_text(str);
      } else {
        // Nothing to do here
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

    self.insert_button.set_enabled(
      !state.dialogue_open && state.witness_dir_okay && state.test_ogg_okay
    );
  }

  // `show_directory_picker` and `show_file_picker` really shoudln't be separate functions but i'm
  // recycling this code from `show_file_picker` in main_window, and duplicating it is easier than
  // taking time to clean it up
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

  // shows a file picker that lets you pick any file, as long as it is an ogg file
  fn show_file_picker(&self, initial_dir: Option<&str>) -> Option<OsString> {
    let mut file_picker: nwg::FileDialog = nwg::FileDialog::default();

    let builder = nwg::FileDialog::builder()
      .title("Select Log File")
      .action(nwg::FileDialogAction::Open)
      .filters("Ogg Vorbis(*.ogg)");
    let builder_result = {
      if let Some(dir) = initial_dir {
        builder
          .default_folder(dir)
          .build(&mut file_picker)
      } else {
        builder.build(&mut file_picker)
      }
    };
      
    if builder_result.is_err() {
      log::error!("Failed to create file picker: {:?}", builder_result);
      return None;
    }

    if file_picker.run(Some(&self.window)) {
      match file_picker.get_selected_item() {
        Ok(result) => {
          Some(result)
        },
        Err(err) => {
          log::error!("File picker failure: {:?}", err);
          None
        },
      }
    } else {
      None
    }
  }
}
