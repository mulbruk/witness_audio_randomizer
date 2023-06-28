use anyhow::{anyhow, Result};
use std::{
  fs,
  io,
  io::{Seek, SeekFrom, Write},
  path::{Path, PathBuf},
};

// ---------------------------------------------------------------------------------------------------
// Sound file headers

// .sound files used by The Witness are just Ogg Vorbis files with an extra 16-byte header preprended.
// The first 12 bytes are [0B 00 00 00 00 00 07 00 00 00 00 00], the final four bytes are the size of
// the Ogg file as an unsigned 32-bit integer, stored litle-endian.

pub fn ogg_to_sound(source_file: &Path, dest_file: &Path) -> Result<()> {
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

pub fn sound_to_ogg(source_file: &Path, dest_file: &Path) -> Result<()> {
  let mut infile  = fs::File::open(source_file)?;
  let mut outfile = fs::File::create(dest_file)?;

  infile.seek(SeekFrom::Start(16))?;
  io::copy(&mut infile, &mut outfile)?;

  Ok(())
}

// ---------------------------------------------------------------------------------------------------
// Things that were useful during the exploratory phase of development

#[allow(dead_code)]
fn search_packages(data_pc: &Path) {
  let pkg_extension = std::ffi::OsStr::new("pkg");

  fs::read_dir(data_pc).unwrap().into_iter()
    .filter_map(|entry| entry.ok())
    .map(|entry| entry.path())
    .filter(|path| path.is_file() && path.extension() == Some(pkg_extension))
    .for_each(|pkg| list_package_sounds(&pkg));
}

#[allow(dead_code)]
fn list_package_sounds(pkg: &Path) {
  let sound_extension = std::ffi::OsStr::new("sound");
  
  let file = fs::File::open(pkg).unwrap();
  let archive = zip::ZipArchive::new(file).unwrap();

  let sound_files: Vec<PathBuf> = archive.file_names()
    .map(|path| PathBuf::from(path))
    .filter(|path| path.extension() == Some(sound_extension))
    .collect();

  if sound_files.len() > 0 {
    println!("{}", pkg.file_name().unwrap().to_string_lossy());
    sound_files.iter()
      .for_each(
        |sound_file| println!("  {}", sound_file.file_name().unwrap().to_string_lossy())
      );
  }
}
