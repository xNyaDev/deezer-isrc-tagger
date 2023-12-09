// Everything is a string here because audio metadata works on strings only

use dialoguer::Select;
use std::error::Error;

use serde_aux::prelude::deserialize_string_from_number;

pub struct Metadata {
    pub title: String,
    pub artists: Vec<Artist>,
    pub album: Album,
    pub bpm: Option<String>,
    pub date: String,
    pub isrc: String,
    pub track_position: String,
}

pub struct Album {
    pub title: String,
    pub artists: Vec<Artist>,
    pub genres: Vec<String>,
    pub number_of_tracks: String,
    pub upc: String,
    pub cover_url: String,
    pub label: String,
}

pub struct Artist {
    pub name: String,
    pub role: ArtistRole,
}

#[derive(Eq, PartialEq)]
pub enum ArtistRole {
    Main,
    Featured,
    Unknown,
}

pub fn get_metadata_from_deezer(isrc: String) -> Result<Metadata, Box<dyn Error>> {
    let reqwest_client = reqwest::blocking::Client::new();
    let track_info = reqwest_client
        .get(format!("https://api.deezer.com/track/isrc:{isrc}"))
        .send()?
        .text()?;
    let track_info = serde_json::from_str::<DeezerTrack>(&track_info)?;

    // Check if there are multiple tracks on Deezer with the same ISRC
    // If there are, prompt the user which to choose
    // I haven't found a better method, as you can't search by ISRC
    let search_results = reqwest_client
        .get(format!(
            "https://api.deezer.com/search?q={}&strict=on",
            urlencoding::encode(&format!(
                "artist:\"{}\" track:\"{}\"",
                track_info.artist.name, track_info.title
            ))
        ))
        .send()?
        .text()?;
    let search_results = serde_json::from_str::<DeezerSearchResults>(&search_results)?;

    let multiple_tracks = search_results
        .data
        .into_iter()
        .filter_map(|searched_track| {
            if track_info.duration == searched_track.duration
                && track_info.title == searched_track.title
            {
                let searched_track_info = reqwest_client
                    .get(format!(
                        "https://api.deezer.com/track/{}",
                        searched_track.id
                    ))
                    .send()
                    .ok()?
                    .text()
                    .ok()?;
                let searched_track_info =
                    serde_json::from_str::<DeezerTrack>(&searched_track_info).ok()?;
                if track_info.isrc == searched_track_info.isrc {
                    Some(searched_track_info)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    let track_info = match multiple_tracks.len() {
        // 0 should never happen but let's just assume it does
        0 => track_info,
        1 => multiple_tracks.into_iter().next().unwrap(),
        _ => {
            let options = multiple_tracks
                .iter()
                .map(|track| format!("Deezer ID: {}, in album: {}", track.id, track.album.title))
                .collect::<Vec<_>>();

            let selection = Select::new()
                .with_prompt("Multiple tracks with the same ISRC found. Choose which one to use")
                .items(&options)
                .interact()?;

            multiple_tracks.into_iter().nth(selection).unwrap()
        }
    };

    let album_info = reqwest_client
        .get(format!(
            "https://api.deezer.com/album/{}",
            track_info.album.id
        ))
        .send()?
        .text()?;
    let album_info = serde_json::from_str::<DeezerAlbum>(&album_info)?;

    Ok(Metadata {
        title: track_info.title,
        artists: track_info
            .contributors
            .into_iter()
            .map(|x| x.into())
            .collect(),
        album: Album {
            title: track_info.album.title,
            artists: album_info
                .contributors
                .into_iter()
                .map(|x| x.into())
                .collect(),
            genres: album_info
                .genres
                .data
                .into_iter()
                .map(|genre| genre.name)
                .collect(),
            number_of_tracks: album_info.nb_tracks,
            upc: album_info.upc,
            cover_url: album_info.cover_xl,
            label: album_info.label,
        },
        bpm: match track_info.bpm.as_str() {
            "0" => None,
            _ => Some(track_info.bpm),
        },
        date: track_info.release_date,
        isrc,
        track_position: track_info.track_position,
    })
}

#[derive(serde::Deserialize)]
struct DeezerSearchResults {
    data: Vec<DeezerSearchResultsTrack>,
}

#[derive(serde::Deserialize)]
struct DeezerSearchResultsTrack {
    #[serde(deserialize_with = "deserialize_string_from_number")]
    id: String,
    title: String,
    #[serde(deserialize_with = "deserialize_string_from_number")]
    duration: String,
}

#[derive(serde::Deserialize)]
struct DeezerTrack {
    #[serde(deserialize_with = "deserialize_string_from_number")]
    id: String,
    title: String,
    isrc: String,
    #[serde(deserialize_with = "deserialize_string_from_number")]
    track_position: String,
    #[serde(deserialize_with = "deserialize_string_from_number")]
    duration: String,
    #[serde(deserialize_with = "deserialize_string_from_number")]
    bpm: String,
    release_date: String,
    album: DeezerTrackAlbum,
    contributors: Vec<DeezerArtist>,
    artist: DeezerArtist,
}

#[derive(serde::Deserialize)]
struct DeezerTrackAlbum {
    #[serde(deserialize_with = "deserialize_string_from_number")]
    id: String,
    title: String,
}

#[derive(serde::Deserialize)]
struct DeezerArtist {
    name: String,
    role: Option<String>,
}

#[derive(serde::Deserialize)]
struct DeezerAlbum {
    upc: String,
    genres: DeezerAlbumGenres,
    label: String,
    #[serde(deserialize_with = "deserialize_string_from_number")]
    nb_tracks: String,
    cover_xl: String,
    contributors: Vec<DeezerArtist>,
}

#[derive(serde::Deserialize)]
struct DeezerAlbumGenres {
    data: Vec<DeezerAlbumGenre>,
}

#[derive(serde::Deserialize)]
struct DeezerAlbumGenre {
    name: String,
}

impl From<String> for ArtistRole {
    fn from(value: String) -> Self {
        match value.as_str() {
            "Main" => ArtistRole::Main,
            "Featured" => ArtistRole::Featured,
            _ => ArtistRole::Unknown,
        }
    }
}

impl From<DeezerArtist> for Artist {
    fn from(value: DeezerArtist) -> Self {
        Artist {
            name: value.name,
            role: if let Some(role) = value.role {
                ArtistRole::from(role)
            } else {
                ArtistRole::Unknown
            },
        }
    }
}
