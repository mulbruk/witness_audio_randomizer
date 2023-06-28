use nwd::NwgUi;
use nwg::NativeUi;
use std::{
  cell::RefCell,
  thread,
};

#[derive(Default, NwgUi)]
pub struct MessageBox {
  message: RefCell<String>,

  #[nwg_control(size: (300, 160), position: (650, 300), title: "", flags: "WINDOW|VISIBLE")]
  #[nwg_events( OnInit: [MessageBox::run], OnWindowClose: [MessageBox::close] )]
  window: nwg::Window,

  #[nwg_control(size: (200, 100), position: (50, 20), text: "", h_align: HTextAlign::Center, v_align: VTextAlign::Center )]
  label: nwg::Label,

  #[nwg_control(size: (64, 32), position: (118, 120), text: "Okay")]
  #[nwg_events( OnButtonClick: [MessageBox::close])]
  close_button: nwg::Button,
}

impl MessageBox {
  pub fn show(message: &str, sender: nwg::NoticeSender) {
    let message = message.to_owned();

    thread::spawn(move || {
      let message = RefCell::new(message);
      let dialogue = MessageBox { message, ..Default::default() };
      let _ui = MessageBox::build_ui(dialogue).expect("Failed to build UI");
      nwg::dispatch_thread_events();
      
      // Notify the main thread that the dialogue completed
      sender.notice();
    });
  }

  fn run(&self) {
    let message = self.message.borrow();
    self.label.set_text(&message);
  }

  fn close(&self) {
    nwg::stop_thread_dispatch();
  }
}
