extern crate native_windows_derive as nwd;
extern crate native_windows_gui as nwg;

use anyhow::{anyhow, Result};
use log;
use nwd::NwgUi;
use nwg::NativeUi;
use rand::{
  thread_rng,
  rngs::StdRng,
  seq::SliceRandom, SeedableRng, RngCore,
};
use rust_embed::RustEmbed;
use serde::{Serialize, Deserialize};
use simplelog;
use std::{
  cell::RefCell,
  collections::{HashMap, hash_map::DefaultHasher},
  ffi::OsString,
  fs,
  hash::Hasher,
  io,
  io::{prelude::*, Read, Write, SeekFrom},
  path::{PathBuf, Path},
  thread,
};
use walkdir::WalkDir;
use zip::write::FileOptions;

//----------------------------------------------------------------------------------------------------
// Configuration

#[derive(Debug, Serialize, Deserialize)]
struct Config {
  witness_dir: PathBuf,
  logs_dir: PathBuf,
}

impl Default for Config {
  fn default() -> Self {
    Config {
      witness_dir: PathBuf::from(r"C:\Program Files\Steam\steamapps\common\The Witness\"),
      logs_dir: std::env::current_dir().unwrap(),
    }
  }
}

impl Config {
  pub fn get() -> Self {
    if let Ok(raw) = fs::read_to_string("config.json") {
      if let Ok(config) = serde_json::from_str(&raw) {
        config
      } else {
          Config::default()
      }
    } else {
      Config::default()
    }
  }

  pub fn save(&self) -> Result<()> {
    let json = serde_json::to_string_pretty(self)?;
    std::fs::write("config.json", json)?;
    Ok(())
  }
}

// ---------------------------------------------------------------------------------------------------
// Back up / unpack data files

fn data_bak_path(witness_dir: &Path) -> PathBuf { witness_dir.join(r"data-pc.zip.bak") }
fn data_dir_path(witness_dir: &Path) -> PathBuf { witness_dir.join(r"data-pc") }
fn data_zip_path(witness_dir: &Path) -> PathBuf { witness_dir.join(r"data-pc.zip") }

fn tmp_dir_path(witness_dir: &Path) -> PathBuf { witness_dir.join(r"tmp") }

fn subtitles_path(witness_dir: &Path) -> PathBuf { witness_dir.join(r"data\strings\en.subtitles") }
fn subs_bak_path(witness_dir: &Path)  -> PathBuf { witness_dir.join(r"data\strings\en.subtitles.bak") }

fn witness_dir_is_okay(witness_dir: &Path) -> bool {
  let data_dir = data_dir_path(witness_dir);
  let data_zip = data_zip_path(witness_dir);

  data_zip.exists() || data_dir.exists()
}

fn backup_and_unpack_audio(witness_dir: &Path) -> Result<()> {
  let data_zip = data_zip_path(witness_dir);
  let data_bak = data_bak_path(witness_dir);
  let data_dir = data_dir_path(witness_dir);
  
  return match ( data_zip.exists(), data_dir.exists() ) {
    (true, false) => {
      unpack(&data_zip, &data_dir)?;
      fs::rename(&data_zip, &data_bak)?;
      Ok(())
    },
    
    (false, true) => {
      Ok(())
    },
    
    (true, true) => {
      if data_bak.exists() {
        fs::remove_file(&data_bak)?;
      } else {
        fs::rename(&data_zip, &data_bak)?;
      }
      Ok(())
    },
    
    (false, false) => {
      Err(anyhow!("Could not back up and unpack data files: {:?} does not exist", data_zip))
    }
  }
}

fn restore_audio_backup(witness_dir: &Path) -> Result<()> {
  let data_bak = data_bak_path(witness_dir);
  let data_dir = data_dir_path(witness_dir);

  if !data_bak.exists() {
    return Err(anyhow!("Could not restore data file backup: {:?} does not exist", data_bak));
  }

  if data_dir.exists() {
    fs::remove_dir_all(&data_dir)?;
  }

  unpack(&data_bak, &data_dir)
}

fn backup_subtitles(witness_dir: &Path) -> Result<()> {
  let subs = subtitles_path(witness_dir);
  let bak  = subs_bak_path(witness_dir);

  if !bak.exists() {
    fs::copy(subs, bak)?;
  }

  Ok(())
}

fn restore_subtitles_backup(witness_dir: &Path) -> Result<()> {
  let subs = subtitles_path(witness_dir);
  let bak  = subs_bak_path(witness_dir);

  fs::copy(bak, subs)?;

  Ok(())
}

fn create_temp_dir(witness_dir: &Path) -> Result<()> {
  let tmp_dir = tmp_dir_path(witness_dir);
  
  if !tmp_dir.exists() {
    fs::create_dir(tmp_dir).map_err(anyhow::Error::from)
  } else if !tmp_dir.is_dir() {
    Err(anyhow!("{:?} already exists and is not a directory", tmp_dir))
  } else {
    Ok(())
  }
}

// ---------------------------------------------------------------------------------------------------
// Unzip / Zip

// This code borrowed and tweaked from the zip crate example at
// https://github.com/zip-rs/zip/blob/master/examples/write_dir.rs
fn pack(source_dir: &Path, dest_file: &Path) -> Result<()> {
  let outfile = fs::File::create(dest_file)?;

  let mut zip = zip::ZipWriter::new(outfile);
  let options = FileOptions::default()
    .compression_method(zip::CompressionMethod::Stored)
    .unix_permissions(0o755);

  let walkdir = WalkDir::new(&source_dir);
  let dir_iter = walkdir.into_iter();

  let mut buffer = Vec::new();
  for entry in dir_iter {
    if let Ok(entry) = entry {
      let path = entry.path();
      let name = path.strip_prefix(&source_dir).unwrap();

      // Write file or directory explicitly
      // Some unzip tools unzip files with directory paths correctly, some do not!
      if path.is_file() {
        // println!("adding file {path:?} as {name:?} ...");
        #[allow(deprecated)]
        zip.start_file_from_path(name, options)?;
        let mut f = fs::File::open(path)?;

        f.read_to_end(&mut buffer)?;
        zip.write_all(&buffer)?;
        buffer.clear();
      } else if !name.as_os_str().is_empty() {
        // Only if not root! Avoids path spec / warning
        // and mapname conversion failed error on unzip
        // println!("adding dir {path:?} as {name:?} ...");
        #[allow(deprecated)]
        zip.add_directory_from_path(name, options)?;
      }
    }
  }
  zip.finish()?;
  Ok(())
}

// This code borrowed and tweaked from the zip crate example at
// https://github.com/zip-rs/zip/blob/master/examples/extract.rs
// fn unpack(source_file: &Path, dest_dir: &Path) -> Result<()> {
fn unpack(source_file: &Path, dest_dir: &Path) -> Result<()> {
  let file = fs::File::open(source_file).unwrap();

  let mut archive = zip::ZipArchive::new(file).unwrap();

  for i in 0..archive.len() {
    let mut file = archive.by_index(i).unwrap();
    let zipped_path = match file.enclosed_name() {
      Some(path) => path,
      None => continue,
    };
    let outpath: PathBuf = [dest_dir, zipped_path].iter().collect();

    if (*file.name()).ends_with('/') {
      // println!("File {} extracted to \"{}\"", i, outpath.display());
      fs::create_dir_all(&outpath).unwrap();
    } else {
      if let Some(p) = outpath.parent() {
        if !p.exists() {
          fs::create_dir_all(p).unwrap();
        }
      }
      let mut outfile = fs::File::create(&outpath).unwrap();
      io::copy(&mut file, &mut outfile).unwrap();
    }
  }

  Ok(())
}

fn extract(source_file: &Path, file_to_extract: &Path, dest_dir: &Path) -> Result<()> {
  let file = fs::File::open(source_file).unwrap();

  let mut archive = zip::ZipArchive::new(file).unwrap();

  for i in 0..archive.len() {
    let mut file = archive.by_index(i).unwrap();

    let outpath: PathBuf = dest_dir.join(&file_to_extract);

    if (*file.name()) == file_to_extract.to_string_lossy() {
      let mut outfile = fs::File::create(&outpath).unwrap();
      io::copy(&mut file, &mut outfile).unwrap();
    }
  }

  Ok(())
}

// ---------------------------------------------------------------------------------------------------
// 

// fn search_packages<P: AsRef<Path>>(data_pc: P) {
//   let pkg_extension = std::ffi::OsStr::new("pkg");

//   fs::read_dir(data_pc).unwrap().into_iter()
//     .filter_map(|entry| entry.ok())
//     .map(|entry| entry.path())
//     .filter(|path| path.is_file() && path.extension() == Some(pkg_extension))
//     .for_each(|pkg| list_package_sounds(&pkg));
// }

// fn list_package_sounds<P: AsRef<Path>>(pkg: P) {
//   let sound_extension = std::ffi::OsStr::new("sound");
  
//   let file = fs::File::open(pkg).unwrap();
//   let archive = zip::ZipArchive::new(file).unwrap();

//   let sound_files: Vec<PathBuf> = archive.file_names()
//     .map(|path| PathBuf::from(path))
//     .filter(|path| path.extension() == Some(sound_extension))
//     .collect();

//   if sound_files.len() > 0 {
//     println!("{}", pkg.file_name().unwrap().to_string_lossy());
//     sound_files.iter()
//       .for_each(
//         |sound_file| println!("  {}", sound_file.file_name().unwrap().to_string_lossy())
//       );
//   }
// }

// ---------------------------------------------------------------------------------------------------
// Sound file headers

// .sound files used by The Witness are just Ogg Vorbis files with an extra 16-byte header preprended.
// The first 12 bytes are [0B 00 00 00 00 00 07 00 00 00 00 00], the final four bytes are the size of
// the Ogg file as an unsigned 32-bit integer, stored litle-endian.

fn ogg_to_sound(source_file: &Path, dest_file: &Path) -> Result<()> {
  let mut infile  = fs::File::open(source_file)?;
  let mut outfile = fs::File::create(dest_file)?;
  
  let infile_size = infile.metadata()?.len();
  if infile_size > u32::MAX.into() {
    return Err(anyhow!("Sound file `{:?}` is too large!", source_file));
  }
  let infile_size_32 = infile_size as u32;

  let header_bytes: Vec<u8> = vec![
    0x0B, 0x00, 0x00, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00, 0x00, 0x00
  ];
  
  outfile.write(&header_bytes)?;
  outfile.write(&infile_size_32.to_le_bytes())?;
  io::copy(&mut infile, &mut outfile)?;

  Ok(())
}

fn sound_to_ogg(source_file: &Path, dest_file: &Path) -> Result<()> {
  let mut infile  = fs::File::open(source_file)?;
  let mut outfile = fs::File::create(dest_file)?;

  infile.seek(SeekFrom::Start(16))?;
  io::copy(&mut infile, &mut outfile)?;

  Ok(())
}

// ---------------------------------------------------------------------------------------------------
// Sound file insertion

#[derive(Debug, Eq, PartialEq, Hash)]
enum SoundDestination {
  Package(PathBuf),
  Root,
}

#[derive(Debug)]
struct SoundInsertion {
  source_file: PathBuf,
  dest_file: PathBuf,
}

fn insert_sound_files(
  files: Vec<SoundInsertion>,
  destination: SoundDestination,
  witness_dir: &Path,
) -> Result<()> {
  match destination {
    SoundDestination::Package(pkg) => insert_sound_packaged(files, pkg, witness_dir),
    SoundDestination::Root => insert_sound_loose(files, witness_dir),
  }
}

fn insert_sound_loose(
  files: Vec<SoundInsertion>,
  witness_dir: &Path,
) -> Result<()> {
  let mut dest_pkg_path = PathBuf::new();
  dest_pkg_path.push(witness_dir);
  dest_pkg_path.push("data-pc");

  for insertion in files {
    let mut file_path = dest_pkg_path.clone();
    file_path.push(insertion.dest_file);
    
    ogg_to_sound(&insertion.source_file, &file_path)?;
  }

  Ok(())
}

fn insert_sound_packaged(
  files: Vec<SoundInsertion>,
  dest_pkg: PathBuf,
  witness_dir: &Path
) -> Result<()> {
  let dest_pkg_stem = PathBuf::from(dest_pkg.file_stem().unwrap()); // TODO

  let mut dest_pkg_path = PathBuf::new();
  dest_pkg_path.push(witness_dir);
  dest_pkg_path.push("data-pc");
  dest_pkg_path.push(dest_pkg);
  
  let mut unpacked_pkg_path = PathBuf::new();
  unpacked_pkg_path.push(witness_dir);
  unpacked_pkg_path.push("tmp");
  unpacked_pkg_path.push(&dest_pkg_stem);
  
  unpack(&dest_pkg_path, &unpacked_pkg_path)?;

  for insertion in files {
    let mut file_path = PathBuf::new();
    file_path.push(witness_dir);
    file_path.push("tmp");
    file_path.push(&dest_pkg_stem);
    file_path.push(insertion.dest_file);
    fs::remove_file(&file_path)?;

    ogg_to_sound(&insertion.source_file, &file_path)?;
  }

  pack(&unpacked_pkg_path, &dest_pkg_path)?;

  fs::remove_dir_all(unpacked_pkg_path)?;

  Ok(())
}

// ---------------------------------------------------------------------------------------------------
// Audio logs data

#[derive(RustEmbed)]
#[folder = "data/"]
pub struct DataStore;

impl DataStore {
  pub fn get_logs() -> Vec<AudioLog> {
    let logs_file = DataStore::get("logs.json").unwrap();
    let s = std::str::from_utf8(logs_file.data.as_ref()).unwrap();

    serde_json::from_str(&s).unwrap()
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioLog {
  package:  Option<PathBuf>,
  filename: PathBuf,
  subtitle: String,
}

fn dump_logs(witness_dir: &Path, dest_dir: &Path, logs: &Vec<AudioLog>, subs: &Vec<Subtitle>) -> Result<()> {
  let subs_hash: HashMap<String, String> = subs.iter()
    .map(|Subtitle {key, val}| (key.clone(), val.clone()))
    .collect();

  for log in logs {
    let subtitle = subs_hash.get(&log.subtitle).unwrap();

    if let Some(package) = &(log.package) {
      let package_path = data_dir_path(witness_dir).join(package);
      extract(&package_path, &log.filename, dest_dir)?;

      let sound_path = dest_dir.join(&log.filename);
      let mut ogg_path = sound_path.clone();
      ogg_path.set_extension("ogg");
      sound_to_ogg(&sound_path, &ogg_path)?;
      fs::remove_file(&sound_path)?;

    } else {
      let file_path = data_dir_path(witness_dir).join(&log.filename);
      let dest_path = dest_dir.join(&log.filename);
      sound_to_ogg(&file_path, &dest_path)?;
    }

    let subs_filename = log.filename.with_extension("sub");
    let subs_path = dest_dir.join(subs_filename);
    let mut outfile = fs::File::create(&subs_path)?;
    outfile.write_all(subtitle.as_bytes())?;
  }

  Ok(())
}

// ---------------------------------------------------------------------------------------------------
// 

#[derive(Debug, Clone)]
pub struct NewLog {
  audio: PathBuf,
  subs:  Option<PathBuf>, 
}

fn get_entries<P: AsRef<Path>>(logs_dir: P) -> Vec<NewLog> {
  let ogg_extension = std::ffi::OsStr::new("ogg");

  fs::read_dir(logs_dir).unwrap().into_iter()
    .filter_map(|entry| entry.ok())
    .map(|entry| entry.path())
    .filter(|path| path.is_file() && path.extension() == Some(ogg_extension))
    .map(|ogg| {
      let mut subs_path = ogg.clone();
      subs_path.set_extension("sub");

      let subs = if subs_path.exists() {
        Some(subs_path)
      } else {
        None
      };

      NewLog {
        audio: ogg,
        subs:  subs,
      }

    })
    .collect()
}

// ---------------------------------------------------------------------------------------------------
// 

type InsertedLogsHash = HashMap<SoundDestination, Vec<SoundInsertion>>;
type InsertedSubsHash = HashMap<String, Option<PathBuf>>;

fn randomize(seed: u64, src_dir: &Path) -> (InsertedLogsHash, InsertedSubsHash) {
  let mut logs_to_insert = get_entries(src_dir);
  let mut logs_data = DataStore::get_logs();

  let insert_count = std::cmp::min( logs_to_insert.len(), logs_data.len() );

  let mut rng = StdRng::seed_from_u64(seed);

  logs_to_insert.shuffle(&mut rng);
  let logs_to_insert_iter = logs_to_insert.into_iter().take(insert_count);

  logs_data.shuffle(&mut rng);
  let logs_data_iter = logs_data.into_iter().take(insert_count);

  let pairs = std::iter::zip(logs_data_iter, logs_to_insert_iter);

  let mut inserted_logs: InsertedLogsHash = HashMap::new();
  let mut inserted_subs: InsertedSubsHash = HashMap::new();
  
  for (dest_log, src_log) in pairs {
    let AudioLog { package, filename, subtitle } = dest_log;
    let NewLog { audio, subs } = src_log;

    let dest_pkg = if let Some(path) = package {
      SoundDestination::Package(path)
    } else {
      SoundDestination::Root
    };

    let insertion = SoundInsertion { source_file: audio, dest_file: filename };

    if inserted_logs.contains_key(&dest_pkg) {
      let d: &mut Vec<SoundInsertion> = inserted_logs.get_mut(&dest_pkg).unwrap(); // TODO tidy this up
      d.push(insertion); 
    } else {
      inserted_logs.insert(dest_pkg, vec![insertion]);
    }

    inserted_subs.insert(subtitle, subs);
  }

  (inserted_logs, inserted_subs)
}

struct Subtitle {
  key: String,
  val: String,
}

fn load_subtitles<P: AsRef<Path>>(path: P) -> Result<Vec<Subtitle>> {
  let raw_subs = std::fs::read_to_string(path)?;
  let raw_chunks = raw_subs.split("\n:");
  
  raw_chunks.map(|chunk| {
    if let Some((key, val)) = chunk.split_once("\r\n") {
      Ok(Subtitle {
        key: key.trim().to_owned(),
        val: val.trim().to_owned(),
      })
    } else {
      Err(anyhow!("Read bad chunk in subs file: {:?}", chunk))
    }
  }).collect()
}

fn compile_subtitles<W: Write>(
  subtitles: Vec<Subtitle>, 
  inserted_subtitles: InsertedSubsHash,
  writeable: &mut W
) -> Result<()> {
  for Subtitle { key, val } in subtitles {
    writeln!(writeable, ": {}", key)?;
    writeln!(writeable, "")?;
    if inserted_subtitles.contains_key(&key) {
      if let Some(path) = inserted_subtitles.get(&key).unwrap() {
        let text = match std::fs::read_to_string(path) {
          Ok(str) => str,
          Err(err) => {
            log::error!("Could not read subs file {:?} - {:?}", path, err);
            String::new()
          }
        };

        writeln!(writeable, "{}", text)?;
      } else {
        writeln!(writeable, "")?;
      }
    } else {
      writeln!(writeable, "{}", val)?;
    }
    writeln!(writeable, "")?;
    writeln!(writeable, "")?;
  }

  Ok(())
}

// ---------------------------------------------------------------------------------------------------
// Randomizer window


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
  fn show(source_dir: &Path, witness_dir: &Path, seed: u64, sender: nwg::NoticeSender) {
    let source_dir = source_dir.to_owned();
    let witness_dir = witness_dir.to_owned();

    thread::spawn(move || {
      let params = RefCell::new(
        RandomizerWindowParams {source_dir, witness_dir, seed}
      );
      let dialogue = RandomizerWindow { params, ..Default::default() };
      let ui = RandomizerWindow::build_ui(dialogue).expect("Failed to build UI");
      nwg::dispatch_thread_events();
      
      // Notify the main thread that the dialogue completed
      sender.notice();
    });
  }

  fn run(&self) {
    let params = self.params.borrow();

    let subs_path = subtitles_path(&params.witness_dir);
    let subs_data = match load_subtitles(&subs_path) {
      Ok(data) => data,
      Err(err) => {
        log::error!("Error loading subtitles file {:?} - {:?}", subs_path, err);
        // TODO maybe show a better indication of this failure
        return;
      }
    };
    
    let (logs, subs) = randomize(params.seed, &params.source_dir);

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

      insert_sound_files(
        vals,
        key,
        &params.witness_dir
      ); // TODO handle failure

      self.progress_bar.advance();
    }

    self.progress_text.set_text("Updating subtitles");
    // Deal with subtitles
    {
      if let Ok(subs_file) = fs::File::create(&subs_path) {
        let mut subs_file = subs_file;
        compile_subtitles(subs_data, subs, &mut subs_file); // handle failure
      } else {
        // handle failure
      }
    }
    self.progress_bar.advance();
    
    self.progress_text.set_text("Complete");
    self.close_button.set_enabled(true);
  }

  fn close(&self) {
    nwg::stop_thread_dispatch();
  }
}

// ---------------------------------------------------------------------------------------------------
// Message box

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
  fn show(message: &str, sender: nwg::NoticeSender) {
    let message = message.to_owned();

    thread::spawn(move || {
      let message = RefCell::new(message);
      let dialogue = MessageBox { message, ..Default::default() };
      let ui = MessageBox::build_ui(dialogue).expect("Failed to build UI");
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

// ---------------------------------------------------------------------------------------------------
// Unpack and backup window

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
  fn show(witness_dir: &Path, sender: nwg::NoticeSender) {
    let dir = witness_dir.to_owned();

    thread::spawn(move || {
      let params = CreateBackupsParams { witness_dir: dir };
      let params = RefCell::new(params);
      let dialogue = CreateBackupsDialogue { params, ..Default::default() };
      let ui = CreateBackupsDialogue::build_ui(dialogue).expect("Failed to build UI");
      nwg::dispatch_thread_events();
      
      // Notify the main thread that the dialogue completed
      sender.notice();
    });
  }

  fn run(&self) {
    let params = self.params.borrow();
    
    let data_zip = data_zip_path(&params.witness_dir);
    let data_dir = data_dir_path(&params.witness_dir);
    let data_bak = data_bak_path(&params.witness_dir);

    self.progress_bar.set_range(0..1);
    self.progress_bar.set_step(1);

    if data_zip.exists() && !data_dir.exists() {
      self.progress_text.set_text("Unpacking data files");
      match unpack(&data_zip, &data_dir) {
        Ok(()) => { ; },
        Err(err) => {
          log::error!("Error unpacking data files: {:?}", err);
        },
      };
      self.progress_bar.advance();
    }

    if data_zip.exists() && !data_bak.exists() {
      self.progress_text.set_text("Backing up data files");
      match fs::rename(&data_zip, &data_bak) {
        Ok(()) => { ; },
        Err(err) => {
          log::error!("Error backing up data files: {:?}", err);
        },
      };
      self.progress_bar.advance();
    }

    self.progress_text.set_text("Data files backed up and unpacked");
    
    self.close_button.set_enabled(true);
  }

  fn close(&self) {
    nwg::stop_thread_dispatch();
  }
}

// ---------------------------------------------------------------------------------------------------
// Restore backups window

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
  fn show(witness_dir: &Path, sender: nwg::NoticeSender) {
    let dir = witness_dir.to_owned();

    thread::spawn(move || {
      let params = RestoreBackupsParams { witness_dir: dir };
      let params = RefCell::new(params);
      let dialogue = RestoreBackupsDialogue { params, ..Default::default() };
      let ui = RestoreBackupsDialogue::build_ui(dialogue).expect("Failed to build UI");
      nwg::dispatch_thread_events();
      
      // Notify the main thread that the dialogue completed
      sender.notice();
    });
  }

  fn run(&self) {
    let params = self.params.borrow();
    let mut success: bool = true;

    self.progress_text.set_text("Restoring subtitles");
    match restore_subtitles_backup(&params.witness_dir) {
      Ok(_) => { ; },
      Err(err) => {
        log::error!("Failure restoring subtitle backups: {:?}", err);
        success = false;
      }
    };
    self.progress_bar.advance_delta(2);

    self.progress_text.set_text("Restoring audio files");
    match restore_audio_backup(&params.witness_dir) {
      Ok(_) => { ; },
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

// ---------------------------------------------------------------------------------------------------
// GUI

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
  fn on_window_load(&self) {
    let cfg = self.config.borrow();
    
    self.witness_dir_input.set_text( cfg.witness_dir.to_str().unwrap() );
    self.logs_dir_input.set_text( cfg.logs_dir.to_str().unwrap() );

    let mut rng = &mut thread_rng();
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
    let mut okay = false;

    {
      let mut state = self.state.borrow_mut();
      okay = witness_dir_is_okay(&dir);
      state.witness_dir_okay = okay;
    }

    if okay {
      let witness_dir = dir;
      let data_dir = data_dir_path(&witness_dir);
      let data_bak = data_bak_path(&witness_dir);

      if !data_dir.exists() || !data_bak.exists() {
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

  fn change_seed_value_input(&self) {
    ;
  }

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
    let subs = match load_subtitles( subtitles_path(&config.witness_dir) ) {
      Ok(val) => val,
      Err(err) => {
        log::error!("Error reading subtitles file: {:?}", err);
        MessageBox::show("Could not dump subtitles file. See `application.log` for details.", self.dialogue_notice.sender());
        return
      }
    };

    match dump_logs(&config.witness_dir, &dest_dir, &logs, &subs) {
      Ok(_) => { ; },
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

    let builder_result = nwg::FileDialog::builder()
      .title("Select Folder")
      .action(nwg::FileDialogAction::OpenDirectory)
      .build(&mut directory_picker);
    if builder_result.is_err() {
      // TODO insert logging statement
      return None;
    }

    if directory_picker.run(Some(&self.window)) {
      match directory_picker.get_selected_item() {
        Ok(result) => {
          Some(result)
        },
        Err(err) => {
          println!("Err: {:?}", err); // TODO replace with logging statement
          None
        },
      }
    } else {
      None
    }
  }

}

// ---------------------------------------------------------------------------------------------------
// Main

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

  let config = RefCell::new( Config::get() );

  nwg::init().expect("Failed to init Native Windows GUI");
  
  let app = RandoGui { config: config, ..Default::default() };
  let _gui = RandoGui::build_ui(app).expect("Failed to build UI");

  nwg::dispatch_thread_events();

  Ok(())
}
