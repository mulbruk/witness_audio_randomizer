use anyhow::{anyhow, Result};
use regex::Regex;
use rust_embed::RustEmbed;
use serde::{Serialize, Deserialize};
use std::{
  collections::HashMap,
  fs,
  io::Write,
  path::{Path, PathBuf},
};

use crate::core::{
  util,
  zip,
};

// ---------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioLog {
  pub package:  Option<PathBuf>,
  pub filename: PathBuf,
  pub subtitle: String,
}

#[derive(Debug, Clone)]
pub struct Subtitle {
  pub key: String,
  pub val: String,
}

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

// ---------------------------------------------------------------------------------------------------
// Data paths

fn data_bak_path(witness_dir: &Path) -> PathBuf { witness_dir.join(r"data-pc.zip.bak") }
fn data_dir_path(witness_dir: &Path) -> PathBuf { witness_dir.join(r"data-pc") }
fn data_zip_path(witness_dir: &Path) -> PathBuf { witness_dir.join(r"data-pc.zip") }

#[allow(dead_code)]
fn tmp_dir_path(witness_dir: &Path) -> PathBuf { witness_dir.join(r"tmp") }

fn subtitles_path(witness_dir: &Path) -> PathBuf { witness_dir.join(r"data\strings\en.subtitles") }
fn subs_bak_path(witness_dir: &Path)  -> PathBuf { witness_dir.join(r"data\strings\en.subtitles.bak") }

pub fn witness_dir_is_okay(witness_dir: &Path) -> bool {
  let data_dir = data_dir_path(witness_dir);
  let data_zip = data_zip_path(witness_dir);

  data_zip.exists() || data_dir.exists()
}

// ---------------------------------------------------------------------------------------------------
// Backing up and restoring data

pub fn data_needs_unpacking(witness_dir: &Path) -> bool {
  let data_zip = data_zip_path(witness_dir);
  let data_dir = data_dir_path(witness_dir);

  data_zip.exists() && !data_dir.exists()
}

pub fn data_needs_backing_up(witness_dir: &Path) -> bool {
  let data_zip = data_zip_path(witness_dir);
  let data_bak = data_bak_path(witness_dir);

  data_zip.exists() && !data_bak.exists()
}

pub fn subtitles_need_backing_up(witness_dir: &Path) -> bool {
  let subs = subtitles_path(witness_dir);
  let subs_bak = subs_bak_path(witness_dir);

  
  subs.exists() && !subs_bak.exists()
}

pub fn unpack_witness_data(witness_dir: &Path) -> Result<()> {
  let data_zip = data_zip_path(witness_dir);
  let data_dir = data_dir_path(witness_dir);

  zip::unpack(&data_zip, &data_dir)
}

pub fn create_audio_backup(witness_dir: &Path) -> Result<()> {
  let data_zip = data_zip_path(witness_dir);
  let data_bak = data_bak_path(witness_dir);

  fs::rename(&data_zip, &data_bak).map_err(anyhow::Error::from)
}

pub fn restore_audio_backup(witness_dir: &Path) -> Result<()> {
  let data_bak = data_bak_path(witness_dir);
  let data_dir = data_dir_path(witness_dir);

  if !data_bak.exists() {
    return Err(anyhow!("Could not restore data file backup: {:?} does not exist", data_bak));
  }

  if data_dir.exists() {
    fs::remove_dir_all(&data_dir)?;
  }

  zip::unpack(&data_bak, &data_dir)
}

pub fn create_subtitles_backup(witness_dir: &Path) -> Result<()> {
  let subs = subtitles_path(witness_dir);
  let bak  = subs_bak_path(witness_dir);

  if !bak.exists() {
    fs::copy(subs, bak)?;
  }

  Ok(())
}

pub fn restore_subtitles_backup(witness_dir: &Path) -> Result<()> {
  let subs = subtitles_path(witness_dir);
  let bak  = subs_bak_path(witness_dir);

  fs::copy(bak, subs)?;

  Ok(())
}

#[allow(dead_code)]
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

pub fn load_subtitles(witness_dir: &Path) -> Result<Vec<Subtitle>> {
  let path = subtitles_path(witness_dir);

  let raw_subs = std::fs::read_to_string(path)?;
  let pattern = Regex::new(r"(?mR)^:").unwrap(); // (?mR) = multi-line mode + CRLF mode
  let raw_chunks = pattern.split(&raw_subs);
  
  raw_chunks.filter(|chunk| !chunk.trim().is_empty()).map(|chunk| {
    if let Some((key, val)) = chunk.split_once("\r\n") {
      Ok(Subtitle {
        key: key.trim().to_owned(),
        val: val.trim().to_owned(),
      })
    } else {
      log::error!("Read empty chunk in subs file: {:?}", chunk);
      Err(anyhow!("Read empty chunk in subs file: {:?}", chunk))
    }
  }).collect()
}

pub fn dump_logs(witness_dir: &Path, dest_dir: &Path, logs: &Vec<AudioLog>, subs: &Vec<Subtitle>) -> Result<()> {
  let subs_hash: HashMap<String, String> = subs.iter()
    .map(|Subtitle {key, val}| (key.clone(), val.clone()))
    .collect();

  for log in logs {
    let subtitle = subs_hash.get(&log.subtitle).unwrap();

    if let Some(package) = &(log.package) {
      let package_path = data_dir_path(witness_dir).join(package);
      zip::extract(&package_path, &log.filename, dest_dir)?;

      let sound_path = dest_dir.join(&log.filename);
      let mut ogg_path = sound_path.clone();
      ogg_path.set_extension("ogg");
      util::sound_to_ogg(&sound_path, &ogg_path)?;
      fs::remove_file(&sound_path)?;

    } else {
      let file_path = data_dir_path(witness_dir).join(&log.filename);
      let dest_path = dest_dir.join(&log.filename);
      util::sound_to_ogg(&file_path, &dest_path)?;
    }

    let subs_filename = log.filename.with_extension("sub");
    let subs_path = dest_dir.join(subs_filename);
    let mut outfile = fs::File::create(&subs_path)?;
    outfile.write_all(subtitle.as_bytes())?;
  }

  Ok(())
}

// ---------------------------------------------------------------------------------------------------
// Inserting audio files

#[derive(Debug, Eq, PartialEq, Hash)]
pub enum SoundDestination {
  Package(PathBuf),
  Root,
}

#[derive(Debug)]
pub struct SoundInsertion {
  pub source_file: PathBuf,
  pub dest_file: PathBuf,
}

pub type SoundInsertionMap = HashMap<SoundDestination, Vec<SoundInsertion>>;

pub fn insert_sound_files(
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
    
    util::ogg_to_sound(&insertion.source_file, &file_path)?;
  }

  Ok(())
}

fn insert_sound_packaged(
  files: Vec<SoundInsertion>,
  dest_pkg: PathBuf,
  witness_dir: &Path
) -> Result<()> {
  let dest_pkg_stem = PathBuf::from(dest_pkg.file_stem().unwrap());

  let mut dest_pkg_path = PathBuf::new();
  dest_pkg_path.push(witness_dir);
  dest_pkg_path.push("data-pc");
  dest_pkg_path.push(dest_pkg);
  
  let mut unpacked_pkg_path = PathBuf::new();
  unpacked_pkg_path.push(witness_dir);
  unpacked_pkg_path.push("tmp");
  unpacked_pkg_path.push(&dest_pkg_stem);
  
  zip::unpack(&dest_pkg_path, &unpacked_pkg_path)?;

  for insertion in files {
    let mut file_path = PathBuf::new();
    file_path.push(witness_dir);
    file_path.push("tmp");
    file_path.push(&dest_pkg_stem);
    file_path.push(insertion.dest_file);
    fs::remove_file(&file_path)?;

    util::ogg_to_sound(&insertion.source_file, &file_path)?;
  }

  zip::pack(&unpacked_pkg_path, &dest_pkg_path)?;

  fs::remove_dir_all(unpacked_pkg_path)?;

  Ok(())
}

// ---------------------------------------------------------------------------------------------------
// Inserting subtitles

pub type SubsInsertionMap = HashMap<String, Option<PathBuf>>;

pub fn insert_subtitles(
  witness_dir: &Path,
  subtitles: Vec<Subtitle>, 
  inserted_subtitles: SubsInsertionMap,
) -> Result<()> {
  let subs_path = subtitles_path(&witness_dir);
  let mut subs_file = fs::File::create(&subs_path)?;
  
  compile_subtitles(subtitles, inserted_subtitles, &mut subs_file)
}

fn compile_subtitles<W: Write>(
  subtitles: Vec<Subtitle>, 
  inserted_subtitles: SubsInsertionMap,
  writeable: &mut W
) -> Result<()> {
  // writeln! emits '\n' as line terminator for all platforms, so we need to explicitly add the
  // carriage return to keep the subs file from getting mangled across multiple randomizations
  for Subtitle { key, val } in subtitles {
    writeln!(writeable, ": {}\r", key)?;
    writeln!(writeable, "\r")?;
    if inserted_subtitles.contains_key(&key) {
      if let Some(path) = inserted_subtitles.get(&key).unwrap() {
        let text = match std::fs::read_to_string(path) {
          Ok(str) => str,
          Err(err) => {
            log::error!("Could not read subs file {:?} - {:?}", path, err);
            String::new()
          }
        };

        writeln!(writeable, "{}\r", text)?;
      } else {
        writeln!(writeable, "\r")?;
      }
    } else {
      writeln!(writeable, "{}\r", val)?;
    }
    writeln!(writeable, "\r")?;
    writeln!(writeable, "\r")?;
  }

  Ok(())
}
