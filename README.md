# ED Installations Dumper Rust

[![GitHub Downloads (all assets, all releases)](https://img.shields.io/github/downloads/Column01/ed_installations_dumper_rust/total?label=Download&style=for-the-badge)](https://github.com/Column01/ed_installations_dumper_rust/releases)

This is a proof of concept rewrite for my [Python program that does the same thing as this one](https://github.com/Column01/ED-Installations-Dumper). I'd prefer if you use the other one but use at your own risk :D (I do plan on supporting this one though too)

## Building from Source

### Install Pre-Requisites

- Install [MongoDB Community Server](https://www.mongodb.com/try/download/community) _(developed on `7.0.5`)_
- Install [rust](https://www.rust-lang.org/tools/install) _(developed on `1.76.0`)_

**Note:** You can get the latest versions for each, but the versions are specified to ensure compatibility

### Building the Program

1. Download the repo:
    - `git clone https://github.com/Column01/ed_installations_dumper_rust.git`
2. Build the executable:
    - `cd ed_installations_dumper_rust`
    - `cargo build --release`
3. Run it
    - The executable was built and placed in the `target/release` folder as `eddn_indexer.exe` or maybe `eddn_indexer`, depends on platform
    - Move it (if you want) to wherever you want to that has enough disk space for the files to be downloaded to (5-10GB)
    - Run it from the terminal using `./eddn_indexer.exe` or whatever file was built earlier
    - Answer the prompts (yes or no questions about what you want to do)

## Additional Notes

### Resource Limitations

If you have a relatively low ish spec system (>16GB of RAM) I wouldn't suggest running this program.

The import section will use up to 7GB of RAM, not to mention that MongoDB by default will use half of your system RAM on its own (you can configure this, do your own research into that :D).

I have 32GB of RAM and I've been okay, you can probably get away with 16GB though if you alter the mongoDB config as stated earlier

### When to skip a step

While you can skip a step of the process, some things will **NOT** work if you do not do them. (notice the formatting, by not work I mean may crash the program!)

For example, if you indexed the webpage (happens by default) but didn't download the files to disk, you CANNOT import them as there are none there to import.

Additionally, the "save file info to json" step was only added to have feature parity to the original project linked at the top. You can safely skip this step, in fact I may remove it later.
