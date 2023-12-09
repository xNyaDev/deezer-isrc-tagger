use std::error::Error;
use std::fs;
use std::path::PathBuf;

use crate::metadata::ArtistRole;
use clap::Parser;
use lofty::{AudioFile, ItemKey, Picture, PictureType, Tag, TaggedFileExt};

mod metadata;

#[derive(Parser)]
#[clap(author, version, about, long_about)]
struct Args {
    /// Audio file to tag and optionally rename
    filename: PathBuf,
    /// A custom ISRC to use instead of reading it from the file
    #[clap(long)]
    isrc: Option<String>,
    /// Remove all other tags from the file except encoding information
    ///
    /// Saved info is: EncodedBy, EncoderSoftware, EncoderSettings, EncodingTime
    #[clap(long)]
    clear: bool,
    /// Rename the file to "$MAIN_ARTISTS - $TITLE [$YEAR]"
    ///
    /// Windows disallowed characters will be changed as if one used encoding
    /// Asterisk,BackSlash,Colon,DoubleQuote,LtGt,Pipe,Question,Slash
    /// in rclone - see https://rclone.org/overview/#encoding
    #[clap(long)]
    rename: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let mut tagged_file = lofty::read_from_path(&args.filename)?;

    let isrc = args.isrc.unwrap_or(
        tagged_file
            .primary_tag()
            .expect("File is not tagged, please pass in an ISRC with --isrc")
            .get(&ItemKey::Isrc)
            .expect("File tags do not have an ISRC, please pass it in with --isrc")
            .value()
            .clone()
            .into_string()
            .expect("File tags do not have an ISRC, please pass it in with --isrc"),
    );

    let encoding_info = tagged_file.primary_tag().map(|primary_tag| {
        [
            primary_tag.get(&ItemKey::EncodedBy).cloned(),
            primary_tag.get(&ItemKey::EncoderSettings).cloned(),
            primary_tag.get(&ItemKey::EncoderSoftware).cloned(),
            primary_tag.get(&ItemKey::EncodingTime).cloned(),
        ]
    });

    if args.clear {
        tagged_file.clear();
    }

    // If there are multiple tracks with the same ISRC on Deezer,
    // this will prompt the user which to choose
    let metadata = metadata::get_metadata_from_deezer(isrc)?;

    let filename = format!(
        "{} - {} [{}]",
        metadata
            .artists
            .iter()
            .filter_map(|artist| if artist.role == ArtistRole::Main {
                Some(artist.name.as_str())
            } else {
                None
            })
            .collect::<Vec<_>>()
            .join(", "),
        &metadata.title,
        &metadata.date[0..=3]
    );
    let filename = filename
        .replace('*', "＊")
        .replace('\\', "＼")
        .replace(':', "：")
        .replace('\"', "＂")
        .replace('<', "＜")
        .replace('>', "＞")
        .replace('|', "｜")
        .replace('?', "？")
        .replace('/', "／");

    if !tagged_file.contains_tag_type(tagged_file.primary_tag_type()) {
        let mut tag = Tag::new(tagged_file.primary_tag_type());

        if let Some(encoding_info) = encoding_info {
            encoding_info.into_iter().flatten().for_each(|tag_item| {
                tag.insert(tag_item);
            });
        }

        tagged_file.insert_tag(tag);
    }

    let tag = tagged_file.primary_tag_mut().unwrap();

    let mut response = reqwest::blocking::get(metadata.album.cover_url)?;
    let mut cover = Picture::from_reader(&mut response)?;
    cover.set_pic_type(PictureType::CoverFront);

    tag.remove_picture_type(PictureType::CoverFront);
    tag.push_picture(cover);

    tag.insert_text(
        ItemKey::AlbumArtist,
        metadata
            .album
            .artists
            .iter()
            .filter_map(|artist| {
                if artist.role == ArtistRole::Main {
                    Some(artist.name.as_str())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join(", "),
    );
    tag.insert_text(ItemKey::AlbumTitle, metadata.album.title);

    tag.insert_text(ItemKey::Barcode, metadata.album.upc);
    if let Some(bpm) = metadata.bpm {
        tag.insert_text(ItemKey::Bpm, bpm);
    }

    tag.insert_text(ItemKey::TrackTitle, metadata.title);
    tag.insert_text(
        ItemKey::TrackArtist,
        metadata
            .artists
            .iter()
            .filter_map(|artist| {
                if artist.role == ArtistRole::Main {
                    Some(artist.name.as_str())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join(", "),
    );

    tag.insert_text(ItemKey::TrackNumber, metadata.track_position);
    tag.insert_text(ItemKey::TrackTotal, metadata.album.number_of_tracks);

    tag.insert_text(ItemKey::Year, metadata.date[0..=3].to_string());
    tag.insert_text(ItemKey::RecordingDate, metadata.date.clone());
    tag.insert_text(ItemKey::OriginalReleaseDate, metadata.date);

    tag.insert_text(ItemKey::Isrc, metadata.isrc);
    tag.insert_text(ItemKey::Label, metadata.album.label);

    tag.insert_text(ItemKey::Genre, metadata.album.genres.join(", "));

    tagged_file.save_to_path(&args.filename)?;

    if args.rename {
        let extension = args
            .filename
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let new_path = args.filename.parent().unwrap();
        let new_path = new_path.join(format!("{filename}.{extension}"));
        fs::rename(args.filename, new_path)?;
    }

    Ok(())
}
