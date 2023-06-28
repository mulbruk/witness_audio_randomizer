use anyhow::Result;
use std::{
  fs,
  io,
  io::{Read, Write},
  path::{Path, PathBuf},
};
use walkdir::WalkDir;
use zip::write::FileOptions;

// This code borrowed and tweaked from the zip crate example at
// https://github.com/zip-rs/zip/blob/master/examples/write_dir.rs
pub fn pack(source_dir: &Path, dest_file: &Path) -> Result<()> {
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
pub fn unpack(source_file: &Path, dest_dir: &Path) -> Result<()> {
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

pub fn extract(source_file: &Path, file_to_extract: &Path, dest_dir: &Path) -> Result<()> {
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
