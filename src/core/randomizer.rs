use rand::{
  rngs::StdRng,
  seq::SliceRandom, SeedableRng,
};
use std::{
  collections::HashMap,
  fs,
  path::{Path, PathBuf},
};

use crate::core::witness_data::{
  AudioLog, DataStore, SoundDestination, SoundInsertion, SoundInsertionMap, SubsInsertionMap
};

pub fn randomize(seed: u64, src_dir: &Path) -> (SoundInsertionMap, SubsInsertionMap) {
  let mut logs_to_insert = get_entries(src_dir);
  let mut logs_data = DataStore::get_logs();

  let insert_count = std::cmp::min( logs_to_insert.len(), logs_data.len() );

  let mut rng = StdRng::seed_from_u64(seed);

  logs_to_insert.shuffle(&mut rng);
  let logs_to_insert_iter = logs_to_insert.into_iter().take(insert_count);

  logs_data.shuffle(&mut rng);
  let logs_data_iter = logs_data.into_iter().take(insert_count);

  let pairs = std::iter::zip(logs_data_iter, logs_to_insert_iter);

  let mut inserted_logs: SoundInsertionMap = HashMap::new();
  let mut inserted_subs: SubsInsertionMap = HashMap::new();
  
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
      let d: &mut Vec<SoundInsertion> = inserted_logs.get_mut(&dest_pkg).unwrap();
      d.push(insertion); 
    } else {
      inserted_logs.insert(dest_pkg, vec![insertion]);
    }

    inserted_subs.insert(subtitle, subs);
  }

  (inserted_logs, inserted_subs)
}

// ---------------------------------------------------------------------------------------------------

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
