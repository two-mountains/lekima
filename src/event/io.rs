use ncmapi::{ResourceType, SearchType};

#[derive(Debug)]
pub enum IoEvent {
    Signin(String, String),
    UserProfile,
    UserPlaylists,
    RecommendedSongs,
    RecommendedPlaylists,
    PlaylistDetail(usize),
    Search(String, SearchType),
    Comments(usize, ResourceType),
    SongUrls(Vec<usize>),
    Fav(usize),
    Lyric(usize),
    UserPodCasts(usize),
    PodcastAudios(usize),
    UserCloud,
    RecentlyPlayed,
    ArtistSublist,
}
