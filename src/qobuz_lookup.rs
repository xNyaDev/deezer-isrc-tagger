use std::error::Error;

use dialoguer::Select;
use serde_aux::prelude::deserialize_default_from_null;

pub fn find_isrc(app_id: &str, album_id: &str) -> Result<String, Box<dyn Error>> {
    let album_info = reqwest::blocking::get(format!(
        "https://www.qobuz.com/api.json/0.2/album/get?app_id={app_id}&album_id={album_id}"
    ))?
    .text()?;
    let album_info = serde_json::from_str::<QobuzAlbum>(&album_info)?;

    match album_info.tracks.items.len() {
        0 => Err(Box::from("Qobuz album has 0 songs")),
        1 => Ok(album_info.tracks.items.into_iter().next().unwrap().isrc),
        _ => {
            let options = album_info
                .tracks
                .items
                .iter()
                .map(|track| {
                    format!(
                        "{} - {}, ISRC: {}",
                        track.performer.name,
                        match &track.version {
                            None => track.title.clone(),
                            Some(version) => format!("{} ({})", track.title, version),
                        },
                        track.isrc
                    )
                })
                .collect::<Vec<_>>();

            let selection = Select::new()
                .with_prompt("Multiple tracks in the album found. Choose which one to use")
                .items(&options)
                .interact()?;

            Ok(album_info
                .tracks
                .items
                .into_iter()
                .nth(selection)
                .unwrap()
                .isrc)
        }
    }
}

#[derive(serde::Deserialize)]
struct QobuzAlbum {
    tracks: QobuzAlbumTracks,
}
#[derive(serde::Deserialize)]
struct QobuzAlbumTracks {
    items: Vec<QobuzAlbumTrackItem>,
}

#[derive(serde::Deserialize)]
struct QobuzAlbumTrackItem {
    title: String,
    #[serde(deserialize_with = "deserialize_default_from_null")]
    version: Option<String>,
    isrc: String,
    performer: QobuzAlbumTrackItemPerformer,
}

#[derive(serde::Deserialize)]
struct QobuzAlbumTrackItemPerformer {
    name: String,
}
