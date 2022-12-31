struct Playlist {
    pub name: String,
    pub description: String,
    pub visibility: Visibility,
    pub followers: u32,
    pub tracks: Vec<Track>,
}

struct Track {
    pub name: String,
    pub artists: Vec<Artist>,
    pub album: Album,
    pub duration: u32,
    pub explicit: bool,
    pub uri: SpotifyURI,
}

struct User {
    pub followers: u32,
    pub uri: SpotifyURI,
    pub name: String,
}

struct Album {
    pub name: String,
    pub artists: Vec<Artist>,
    pub uri: SpotifyURI,
}

struct Artist {
    pub name: String,
    pub uri: SpotifyURI,
}

struct SpotifyURI {
    pub uri: String,
}

impl SpotifyURI {
    pub fn from_str(uri: String) -> SpotifyURI {
        SpotifyURI { uri }
    }
}

enum Visibility {
    Public,
    Private,
    Collaborative,
}

impl Visibility {
    fn from_api(collaborative: bool, public: bool) -> Visibility {
        if collaborative {
            Visibility::Collaborative
        } else if public {
            Visibility::Public
        } else {
            Visibility::Private
        }
    }
}
