use ncm_api_rs::{ApiClient, create_client};
use once_cell::sync::Lazy;
use std::{any, sync::RwLock};

use crate::api::{ SoundQuality, album_subscribe, get_album_detail, get_artist_album, get_artist_detail, get_artist_mv, get_artist_song, get_lryic, get_playlist_detail, get_recommend_playlist, get_recommend_song, get_song_detail, get_song_url, get_user_detail, get_user_info, get_user_playlist, get_user_playlist_collected, get_user_playlist_created, get_user_sub_album, get_user_subcount, is_like_song, like_song, model::{AlbumDetail, LyricDetail, UserInfo}, playlist_create, playlist_delete, playlist_subscribe, playlist_track_add, playlist_track_del};

static CLIENT: Lazy<RwLock<Option<ApiClient>>> = Lazy::new(|| RwLock::new(None));

pub fn init_client(cookie: String) {
    let client = create_client(Some(cookie));

    let mut guard = CLIENT.write().unwrap();
    *guard = Some(client);
}

pub fn client() -> ApiClient {
    CLIENT
        .read()
        .unwrap()
        .as_ref()
        .expect("NCM client not initialized")
        .clone()
}





#[tokio::test]
async fn test_init_client() {
    init_client("MUSIC_A_T=1628302039878; MUSIC_R_T=1628302040015; MUSIC_R_U=00236B77FCA4628CDCF272DA9C15003D5625364D3CF83134A651ECA863E4E450AB1F9FE6AD2D3FAE60EC080DA0E16D8F5AFB8A871D7F8D775B2F64C3C883E111C03C8821B4449EFA1D677C5EE50978A86B; NMTID=00OJQaCFkk3ieiCe0XTuG1gnl2rEPAAAAGdbBUBAw; __csrf=a1a3d17aaafeeb01ea60cf5871667fba; MUSIC_U=005D16BA9075E5D9A048A0F7962D4C91FA1AEB0F61D1E69F3B7734E57AB813091A11E5A04165D52A6419E40048509923D5460F18BEC14FBC83A56E77BD27DD892328AD08D8C12C824EDF0F154EDA47FF32A41C75257DAFBA7C7A22BC4C4482C94B17828718E6822BFCC4DA07043E3F71C640F4D2F41DFF9B3A67DA615CD38A1E7BC135089E8EE5E73438EA3FAC2AB441092900C3F5ECD6CB8A09B0456597D32B228064168E04FF074199C19A9EC7164CAEA611C99C611E5043EE29A744FD8B4D0D6BBF41045385C1744FFF5FA06C10169DD11B83FE5E4AA6F87284B6EEF2C915DB7856DA8CE1FEC337A6EC6662A2195F9B328B04B592587866C2B4F86BB89022F0DAEABE29DD8FEF0C61D92BE4D12CA312FE3833C9B8D053E326BA5B44EA8BAFA65BA396D7BFA87FF174733D3F44D8A40A3AA931E258985D6552C8CF0F97F30BA1E91BCB9F94AF5FFF958F608F3BB4A762450222DD094C55262805A809B9430734DD501D646E7561F43F5E69C812F85231; Max-Age=2147483647; Expires=Mon, 26 Apr 2094 11:07:36 GMT; Path=/wapi/clientlog".to_string());

    // test_playlist_detail().await;
    // test_recommend_playlist().await;
    // test_song_url().await;
    // test_song_detail().await;

    // test_album_detail().await;
    // test_recommend_songs().await;
    // test_lyric().await;
    // test_user_info().await;
    // test_user_subcount().await;
    // test_user_playlist().await;
    // test_user_sub_album().await;
    // test_user_detail().await;
    // test_artist_detail().await;
    // test_artist_song().await;
    // test_artist_album().await;
    // test_artist_mv().await;

    // test_playlist_create_and_delete().await;
    // test_playlist_subscribe().await;
    // test_like_song().await;
    // test_collect_song().await;
    test_album_sub().await;

}

async fn test_album_sub(){
    match album_subscribe(32311, true).await {
        Ok(_) => println!("Album subscribed successfully!"),
        Err(e) => eprintln!("Error subscribing to album: {}", e),
    }

    match album_subscribe(32311, false).await {
        Ok(_) => println!("Album unsubscribed successfully!"),
        Err(e) => eprintln!("Error unsubscribing to album: {}", e),
    }
}

async fn test_collect_song(){
    match playlist_track_add(17922927485, 1969519579).await {
        Ok(_) => println!("Song collected successfully!"),
        Err(e) => eprintln!("Error collecting song: {}", e),
    }

    match playlist_track_del(17922927485,1969519579).await {
        Ok(_) => println!("Song uncollected successfully!"),
        Err(e) => eprintln!("Error uncollecting song: {}", e),
    }

}
async fn test_like_song(){
    match like_song(1969519579, true).await {
        Ok(_) => println!("Song liked successfully!"),
        Err(e) => eprintln!("Error liking song: {}", e),
    }

    match is_like_song(1969519579).await{
        Ok(liked) => println!("Song is liked: {}", liked),
        Err(e) => eprintln!("Error checking song like status: {}", e),
    }


    match like_song(1969519579, false).await {
        Ok(_) => println!("Song unliked successfully!"),
        Err(e) => eprintln!("Error unliking song: {}", e),
    }

    match is_like_song(1969519579).await{
        Ok(liked) => println!("Song is liked: {}", liked),
        Err(e) => eprintln!("Error checking song like status: {}", e),
    }
}

async fn test_playlist_create_and_delete(){
    match playlist_create("test").await {
        Ok(id) => {
            println!("Playlist created with ID: {}", id);
            match playlist_delete(id).await {
                Ok(_) => println!("Playlist deleted successfully!"),
                Err(e) => eprintln!("Error deleting playlist: {}", e),
            }
        },
        Err(e) => eprintln!("Error creating playlist: {}", e),
    }
}

async fn test_playlist_subscribe(){
    match playlist_subscribe(2226641834, true).await {
        Ok(_) => println!("Playlist subscribed successfully!"),
        Err(e) => eprintln!("Error subscribing to playlist: {}", e),
    }

    match playlist_subscribe(2226641834, false).await {
        Ok(_) => println!("Playlist unsubscribed successfully!"),
        Err(e) => eprintln!("Error unsubscribing to playlist: {}", e),
    }
}

async fn test_artist_mv(){
    match get_artist_mv(11972054).await {
        Ok(mvs) => println!("Mvs: {:?}", mvs),
        Err(e) => eprintln!("Error fetching artist mvs: {}", e),
    }

}
async fn test_artist_album(){
    match get_artist_album(11972054).await {
        Ok(albums) => println!("Albums: {:?}", albums),
        Err(e) => eprintln!("Error fetching artist albums: {}", e),
    }
}

async fn test_artist_song(){
    match get_artist_song(11972054).await {
        Ok(songs) => println!("Songs: {:?}", songs),
        Err(e) => eprintln!("Error fetching artist songs: {}", e),
    }
}
async fn test_artist_detail() {
    match get_artist_detail(11972054).await {
        Ok(detail) => println!("Artist Detail: {:?}", detail),
        Err(e) => eprintln!("Error fetching artist detail: {}", e),
    }
}

async fn test_playlist_detail() {
    match get_playlist_detail(3136952023).await {
        Ok(detail) => println!("Playlist Detail: {:?}", detail),
        Err(e) => eprintln!("Error fetching playlist detail: {}", e),
    }
}

async fn test_recommend_playlist() {
    match get_recommend_playlist().await {
        Ok(playlists) => println!("Recommended Playlists: {:?}", playlists),
        Err(e) => eprintln!("Error fetching recommended playlists: {}", e),
    }
}

async fn test_song_url() {
    match get_song_url(1969519579, SoundQuality::Standard).await {
        Ok(url) => println!("Song URL: {}", url),
        Err(e) => eprintln!("Error fetching song URL: {}", e),
    }
}

async fn test_song_detail() {
    match get_song_detail(vec![3363002263, 2003647821]).await {
        Ok(songs) => println!("Songs: {:?}", songs),
        Err(e) => eprintln!("Error fetching song details: {}", e),
    }
}

async fn test_album_detail() {
    match get_album_detail(32311).await {
        Ok(detail) => println!("Album Detail: {:?}", detail),
        Err(e) => eprintln!("Error fetching album detail: {}", e),
    }
}

async fn test_recommend_songs() {
    match get_recommend_song().await {
        Ok(songs) => println!("Recommended Songs: {:?}", songs),
        Err(e) => eprintln!("Error fetching recommended songs: {}", e),
    }
}

async fn test_lyric() {
    match get_lryic(1824020871).await {
        Ok(lyric) => println!("Lyric: {:?}", lyric),
        Err(e) => eprintln!("Error fetching lyric: {}", e),
    }

    match get_lryic(1831761295).await {
        Ok(lyric) => println!("Lyric: {:?}", lyric),
        Err(e) => eprintln!("Error fetching lyric: {}", e),
    }

    match get_lryic(1923203310).await {
        Ok(lyric) => println!("Lyric: {:?}", lyric),
        Err(e) => eprintln!("Error fetching lyric: {}", e),
    }

    match get_lryic(1966112058).await {
        Ok(lyric) => println!("Lyric: {:?}", lyric),
        Err(e) => eprintln!("Error fetching lyric: {}", e),
    }
}


async fn test_user_info() {
    match get_user_info().await {
        Ok(info) => println!("User Info: {:?}", info),
        Err(e) => eprintln!("Error fetching user info: {}", e),
    }
}

async fn test_user_subcount() {
    match get_user_subcount().await {
        Ok(counts) => println!("User Counts: {:?}", counts),
        Err(e) => eprintln!("Error fetching user counts: {}", e),
    }
}

async fn test_user_playlist() {
    match get_user_playlist(5128948380).await {
        Ok(playlists) => println!("User Playlists: {:?}", playlists),
        Err(e) => eprintln!("Error fetching user playlists: {}", e),
    }

    match get_user_playlist_created(5128948380).await {
        Ok(playlists) => println!("User Created Playlists: {:?}", playlists),
        Err(e) => eprintln!("Error fetching user created playlists: {}", e),
    }

    match get_user_playlist_collected(5128948380).await {
        Ok(playlists) => println!("User Collected Playlists: {:?}", playlists),
        Err(e) => eprintln!("Error fetching user collected playlists: {}", e),
    }
}

async fn test_user_sub_album() {
    match get_user_sub_album().await {
        Ok(albums) => println!("User Subscribed Albums: {:?}", albums),
        Err(e) => eprintln!("Error fetching user subscribed albums: {}", e),
    }
}

async fn test_user_detail() {
    match get_user_detail(5128948380).await {
        Ok(details) => println!("User Details: {:?}", details),
        Err(e) => eprintln!("Error fetching user details: {}", e),
    }
}