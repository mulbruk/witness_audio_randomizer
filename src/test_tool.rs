#![windows_subsystem = "windows"]

extern crate native_windows_derive as nwd;
extern crate native_windows_gui as nwg;

use anyhow::Result;
use log;
use simplelog;
use std::fs;

mod core;
use crate::core::config::Config;

mod gui;
use gui::TestToolGui;

// ---------------------------------------------------------------------------------------------------

fn main() -> Result<()> {
  simplelog::WriteLogger::init(
    simplelog::LevelFilter::Info,
    simplelog::Config::default(),
    fs::OpenOptions::new()
      .create(true)
      .write(true)
      .append(true)
      .open("app.log")
      .unwrap()
  )?;

  log::info!("Application started!");

  let config = Config::get();

  nwg::init().expect("Failed to init Native Windows GUI");
  
  let _gui = TestToolGui::start(config);

  nwg::dispatch_thread_events();

  Ok(())
}
