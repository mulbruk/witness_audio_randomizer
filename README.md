# The Witness Audio Randomizer

**Version 1.0: 'How many of you here have personally witnessed a total eclipse of the sun?' Edition (2024-04-08)**

[![Video](https://img.youtube.com/vi/m_49375h2LY/hqdefault.jpg)](https://www.youtube.com/watch?v=m_49375h2LY)

## What is it?

Enjoy `The Witness`? Sick of all the in-game audio logs that were recorded to bolster jblow's impression of being more clever than he is? `The Witness Audio Randomizer` may be just the tool for you! This tool allows you to (1) insert new (better!) audio logs into the game (randomly!), and (2) extract all audio logs and subtitles from the game to a folder of your choice (if you want to, I guess).

To help you get started, the randomizer comes with a starter pack of six audio logs:

- The Bouncer: _THE BOUNCER_
- Brian Moriarty: _The Secret of Psalm 46_
- The Looker: _Gravity's Rainbow_
- The Looker: _Invisible Cities II_
- The Looker: _Parable of the Ship_
- Soulja Boy: _Commentary on Braid_

You can stick these into the game, or just use them as a reference for the audio/subtitle file formats required by the game if you want to create your own set of logs.

## Installation

### Pre-built binary

[Download the latest version from the releases page](https://github.com/mulbruk/witness_audio_randomizer/releases)

### From source

Building from source requires you to have the [Rust compiler toolchain](https://www.rust-lang.org/tools/install) installed on your system.

```
git clone https://github.com/mulbruk/witness_audio_randomizer.git
cargo build --release
```


## Usage Guide

### Audio Log Randomizer

![Audio randomizer interface](https://raw.githubusercontent.com/mulbruk/witness_audio_randomizer/main/audio_randomizer.png "The Witness Audio Randomizer")

1) **Witness directory**:  
The location in which The Witness is installed. Default location is `C:\Program Files\Steam\steamapps\common\The Witness\`. After selecting a directory in which the game files are detected, a backup will be created of the data files affected by the randomizer. This backup will use about 2.5GB of space.
2) **Seed value**:  
The seed value used for randomization. It doesn't really matter what value is used. ヽ(ー_ー )ノ
3) **Audio logs directory**:  
The location in which the audio and (optional) subtitle files you wish to insert are located.
4) **Restore data files**:  
Restores the backed up files and returns the game to its original state.
5) **I'm feeling lucky**:  
Press only if you feel lucky!	(=^ ◡ ^=)
Will rearrange the audio logs using data files from the game. Useful if you don't have any (or very few) custom audio logs to insert.
6) **Randomize**:  
Randomly insert the selected audio logs into The Witness.
7) **Dump audio logs**:  
Extracts all audio logs and subtitles from the game's data files to a location of your choosing.

### Test Tool

![Test tool interface](https://raw.githubusercontent.com/mulbruk/witness_audio_randomizer/main/test_tool.png "Test Tool")

The test tool allows you to insert a chosen audio log/subtitle pair into the game as the mountaintop audio log.

1) **Witness directory**:  
The location in which The Witness is installed. Default location is `C:\Program Files\Steam\steamapps\common\The Witness\`. After selecting a directory in which the game files are detected, a backup will be created of the data files affected by the randomizer.
2) **Test file path**:  
Location of the `.ogg` file to insert. If a `.sub` file with the same name is located in the same directory as the `.ogg`, it will be inserted as well.
3) **Insert log**:  
Insert the selected audio log file. The log will replace the mountaintop log in the game.
