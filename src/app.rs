use std::sync::mpsc::Sender;

use ncmapi::{NcmApi, ResourceType, SearchType, types::{Album, Playlist, RecommendedSongs, ResourceComments, Song, UserProfile}};
use serde_json::Value;

use crate::{event::IoEvent, player::{AudioPlayer, LAudioPlayer, PlaybackContext}};

struct AppConfig {}

pub(crate) enum SearchResult {
    Song,
    Album,
    Artist,
    Playlist,
    Podcast,
}

pub struct App {
    config: AppConfig,

    player: Box<dyn AudioPlayer>,
    playback_context: Option<PlaybackContext>,

    user: Option<UserProfile>,
    fm: Option<Vec<Song>>,
    cloud: Option<Vec<Song>>,
    user_playlists: Option<Vec<Playlist>>,
    user_fav_playlists: Option<Vec<Playlist>>,
    user_fav_albums: Option<Vec<Album>>,
    // user_fav_artists: Option<Vec<Artist>>,
    recommended_songs: Option<RecommendedSongs>,
    recommended_playlists: Option<Vec<Playlist>>,
    recently_played: Option<Vec<Song>>,
    comments: Option<Vec<ResourceComments>>,
    search_limit: u8,
    search_results: Option<SearchResult>,
    selected_playlist_index: usize,
    active_playlist_index: Option<usize>,
    seek_ms: Option<u128>,
    track_table: Option<Vec<Song>>,
    // artist_table: Option<Vec<Song>>,
    track_table_index: usize,

    loading: bool,
    // logged in or not
    auth: bool,

    io_tx: Option<Sender<IoEvent>>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            config: AppConfig {},
            playback_context: None,
            player: Box::new(LAudioPlayer::try_new().unwrap()),
            user: None,
            fm: None,
            cloud: None,
            user_playlists: None,
            user_fav_albums: None,
            user_fav_playlists: None,
            recommended_songs: None,
            recommended_playlists: None,
            recently_played: None,
            comments: None,
            search_limit: 20,
            search_results: None,
            selected_playlist_index: 0,
            active_playlist_index: None,
            seek_ms: None,
            track_table: None,
            track_table_index: 0,

            loading: false,
            io_tx: None,
            auth: false,
            //
        }
    }
}

impl App {
    pub fn new(player: Box<dyn AudioPlayer>, io_tx: Sender<IoEvent>) -> Self {
        Self {
            player,
            io_tx: Some(io_tx),
            ..Self::default()
        }
    }

    pub fn set_player(mut self, player: Box<dyn AudioPlayer>) -> Self {
        self.player = player;
        self
    }

    // network
    fn dispatch(&self, action: IoEvent) {
        if let Some(io_tx) = &self.io_tx {
            if let Err(e) = io_tx.send(action) {
                println!("dispatch io event error: {:?}", e);
            }
        }
    }

    pub fn signin(&self, phone: String, passwd: String) {
        self.dispatch(IoEvent::Signin(phone, passwd));
    }

    pub fn user(&self) {
        self.dispatch(IoEvent::UserProfile);
    }

    pub fn user_playlists(&self) {
        self.dispatch(IoEvent::UserPlaylists);
    }

    pub fn recommended_songs(&self) {
        self.dispatch(IoEvent::RecommendedSongs);
    }

    pub fn recommended_playlists(&self) {
        self.dispatch(IoEvent::RecommendedPlaylists);
    }

    pub fn playlist_detail(&self, id: usize) {
        self.dispatch(IoEvent::PlaylistDetail(id));
    }

    pub fn search(&self, key: String, t: SearchType) {
        self.dispatch(IoEvent::Search(key, t));
    }

    pub fn comments(&self, id: usize, t: ResourceType) {
        self.dispatch(IoEvent::Comments(id, t));
    }

    pub fn song_urls(&self, ids: Vec<usize>) {
        self.dispatch(IoEvent::SongUrls(ids));
    }

    pub fn fav(&self, id: usize) {
        self.dispatch(IoEvent::Fav(id));
    }

    pub fn lyric(&self, id: usize) {
        self.dispatch(IoEvent::Lyric(id));
    }

    pub fn user_podcasts(&self, id: usize) {
        self.dispatch(IoEvent::UserPodCasts(id));
    }

    pub fn podcast_audios(&self, id: usize) {
        self.dispatch(IoEvent::PodcastAudios(id));
    }

    pub fn user_cloud(&self) {
        self.dispatch(IoEvent::UserCloud);
    }

    pub fn recently_played(&self) {
        self.dispatch(IoEvent::RecentlyPlayed);
    }

    pub fn artist_sublist(&self) {
        self.dispatch(IoEvent::ArtistSublist);
    }
}

#[cfg(test)]
mod tests {}
