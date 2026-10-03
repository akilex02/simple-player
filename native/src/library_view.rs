use crate::library::Song;
use crate::state::{ArtistGroup, SortDirection, SortField};
use std::collections::HashMap;
use std::sync::Arc;

/// Índices (en `songs`) de lo que muestra la tabla: filtrado por artista y
/// búsqueda con scoring tipo "Strawberry" (tokeniza por espacios, suma puntos
/// por coincidencia en título/artista/álbum, descarta lo que no matchea todos
/// los tokens y ordena por relevancia), o bien ordenado por columna si no hay búsqueda.
/// Nombre con que se agrupan las canciones sin artista.
pub const UNKNOWN_ARTIST: &str = "Artista Desconocido";

pub fn filter_sort(
    songs: &[Song],
    query: &str,
    artist: Option<&str>,
    album: Option<&str>,
    field: SortField,
    dir: SortDirection,
) -> Vec<usize> {
    let base = songs
        .iter()
        .enumerate()
        .filter(|(_, s)| artist.map_or(true, |a| artist_matches(s, a)) && album.map_or(true, |al| s.album == al));

    let query = query.trim().to_lowercase();
    if !query.is_empty() {
        let tokens: Vec<&str> = query.split_whitespace().collect();
        let mut scored: Vec<(i32, usize)> = base
            .filter_map(|(i, s)| score(s, &tokens).map(|sc| (sc, i)))
            .collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0));
        return scored.into_iter().map(|(_, i)| i).collect();
    }

    let mut result: Vec<usize> = base.map(|(i, _)| i).collect();
    result.sort_by(|&a, &b| {
        let (a, b) = (&songs[a], &songs[b]);
        let cmp = match field {
            SortField::Title => a.title.to_lowercase().cmp(&b.title.to_lowercase()),
            SortField::Artist => a.artist.to_lowercase().cmp(&b.artist.to_lowercase()),
            SortField::Album => a.album.to_lowercase().cmp(&b.album.to_lowercase()),
            SortField::Duration => a.duration_secs.cmp(&b.duration_secs),
        };
        match dir {
            SortDirection::Asc => cmp,
            SortDirection::Desc => cmp.reverse(),
        }
    });
    result
}

fn artist_matches(song: &Song, wanted: &str) -> bool {
    song.artist == wanted || (wanted == UNKNOWN_ARTIST && song.artist.trim().is_empty())
}

fn score(s: &Song, tokens: &[&str]) -> Option<i32> {
    let title = s.title.to_lowercase();
    let artist = s.artist.to_lowercase();
    let album = s.album.to_lowercase();
    if !tokens.iter().all(|t| title.contains(t) || artist.contains(t) || album.contains(t)) {
        return None;
    }
    let mut score = 0;
    for tok in tokens {
        if title.starts_with(tok) {
            score += 10;
        } else if title.contains(&format!(" {tok}")) {
            score += 8;
        } else if title.contains(tok) {
            score += 5;
        }
        if artist.starts_with(tok) {
            score += 9;
        } else if artist.contains(tok) {
            score += 4;
        }
        if album.starts_with(tok) {
            score += 7;
        } else if album.contains(tok) {
            score += 3;
        }
    }
    Some(score)
}

pub fn group_artists(songs: &[Song], query: &str, dir: SortDirection) -> Vec<ArtistGroup> {
    let query = query.trim().to_lowercase();
    let tokens: Vec<&str> = query.split_whitespace().collect();

    let mut map: HashMap<String, Vec<&Song>> = HashMap::new();
    for song in songs {
        if !tokens.is_empty() {
            let (t, a, al) = (song.title.to_lowercase(), song.artist.to_lowercase(), song.album.to_lowercase());
            if !tokens.iter().all(|tok| t.contains(tok) || a.contains(tok) || al.contains(tok)) {
                continue;
            }
        }
        let artist = if song.artist.trim().is_empty() {
            UNKNOWN_ARTIST.to_string()
        } else {
            song.artist.clone()
        };
        map.entry(artist).or_default().push(song);
    }

    let mut groups: Vec<ArtistGroup> = map
        .into_iter()
        .map(|(artist, songs)| ArtistGroup {
            artist,
            count: songs.len(),
            representative_cover: songs.iter().find_map(|s| s.cover_art.clone()),
        })
        .collect();
    groups.sort_by(|a, b| {
        let cmp = a.artist.to_lowercase().cmp(&b.artist.to_lowercase());
        match dir {
            SortDirection::Asc => cmp,
            SortDirection::Desc => cmp.reverse(),
        }
    });
    groups
}

/// Un álbum se identifica por (álbum, artista): muchas bibliotecas repiten el
/// mismo nombre de álbum en artistas distintos.
#[derive(Debug)]
pub struct AlbumGroup {
    pub album: String,
    pub artist: String,
    pub count: usize,
    pub total_secs: u64,
    pub cover: Option<String>,
}

pub fn group_albums(songs: &[Song], query: &str, dir: SortDirection) -> Vec<AlbumGroup> {
    let query = query.trim().to_lowercase();
    let tokens: Vec<&str> = query.split_whitespace().collect();

    let mut map: HashMap<(String, String), AlbumGroup> = HashMap::new();
    for song in songs {
        if !tokens.is_empty() && !matches_all(song, &tokens) {
            continue;
        }
        let group = map.entry((song.album.clone(), song.artist.clone())).or_insert_with(|| AlbumGroup {
            album: song.album.clone(),
            artist: song.artist.clone(),
            count: 0,
            total_secs: 0,
            cover: None,
        });
        group.count += 1;
        group.total_secs += song.duration_secs;
        if group.cover.is_none() {
            group.cover = song.cover_art.clone();
        }
    }

    let mut groups: Vec<AlbumGroup> = map.into_values().collect();
    groups.sort_by(|a, b| {
        let cmp = (a.album.to_lowercase(), a.artist.to_lowercase()).cmp(&(b.album.to_lowercase(), b.artist.to_lowercase()));
        match dir {
            SortDirection::Asc => cmp,
            SortDirection::Desc => cmp.reverse(),
        }
    });
    groups
}

fn matches_all(song: &Song, tokens: &[&str]) -> bool {
    let (t, a, al) = (song.title.to_lowercase(), song.artist.to_lowercase(), song.album.to_lowercase());
    tokens.iter().all(|tok| t.contains(tok) || a.contains(tok) || al.contains(tok))
}

#[derive(PartialEq)]
struct SongsKey {
    version: u64,
    query: String,
    artist: Option<String>,
    album: Option<String>,
    field: SortField,
    dir: SortDirection,
}

#[derive(PartialEq)]
struct GroupsKey {
    version: u64,
    query: String,
    dir: SortDirection,
}

/// Vista cacheada de la biblioteca: recalcula orden, filtro y agrupación solo
/// cuando cambia alguna de sus entradas, no en cada frame.
#[derive(Default)]
pub struct LibraryView {
    songs_key: Option<SongsKey>,
    songs: Arc<Vec<usize>>,
    groups_key: Option<GroupsKey>,
    groups: Arc<Vec<ArtistGroup>>,
    albums_key: Option<GroupsKey>,
    albums: Arc<Vec<AlbumGroup>>,
    pub recomputes: u32,
}

impl LibraryView {
    pub fn songs(
        &mut self,
        songs: &[Song],
        version: u64,
        query: &str,
        artist: Option<&str>,
        album: Option<&str>,
        field: SortField,
        dir: SortDirection,
    ) -> Arc<Vec<usize>> {
        let key = SongsKey {
            version,
            query: query.trim().to_string(),
            artist: artist.map(String::from),
            album: album.map(String::from),
            field,
            dir,
        };
        if self.songs_key.as_ref() != Some(&key) {
            self.songs = Arc::new(filter_sort(songs, query, artist, album, field, dir));
            self.songs_key = Some(key);
            self.recomputes += 1;
        }
        Arc::clone(&self.songs)
    }

    pub fn albums(&mut self, songs: &[Song], version: u64, query: &str, dir: SortDirection) -> Arc<Vec<AlbumGroup>> {
        let key = GroupsKey { version, query: query.trim().to_string(), dir };
        if self.albums_key.as_ref() != Some(&key) {
            self.albums = Arc::new(group_albums(songs, query, dir));
            self.albums_key = Some(key);
            self.recomputes += 1;
        }
        Arc::clone(&self.albums)
    }

    pub fn groups(&mut self, songs: &[Song], version: u64, query: &str, dir: SortDirection) -> Arc<Vec<ArtistGroup>> {
        let key = GroupsKey { version, query: query.trim().to_string(), dir };
        if self.groups_key.as_ref() != Some(&key) {
            self.groups = Arc::new(group_artists(songs, query, dir));
            self.groups_key = Some(key);
            self.recomputes += 1;
        }
        Arc::clone(&self.groups)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn song(title: &str, artist: &str, album: &str, secs: u64, cover: Option<&str>) -> Song {
        Song {
            path: format!("/m/{title}.mp3"),
            title: title.into(),
            artist: artist.into(),
            album: album.into(),
            duration_secs: secs,
            cover_art: cover.map(String::from),
        }
    }

    fn library() -> Vec<Song> {
        vec![
            song("Zeta", "Metallica", "Black", 300, None),
            song("alfa", "AC/DC", "Back in Black", 200, Some("/c/acdc.jpg")),
            song("Beta", "Metallica", "Load", 250, Some("/c/met.jpg")),
            song("Gamma", "", "Sin album", 100, None),
        ]
    }

    fn titles(songs: &[Song], idx: &[usize]) -> Vec<String> {
        idx.iter().map(|i| songs[*i].title.clone()).collect()
    }

    #[test]
    fn ordena_por_titulo_sin_distinguir_mayusculas() {
        let l = library();
        let idx = filter_sort(&l, "", None, None, SortField::Title, SortDirection::Asc);
        assert_eq!(titles(&l, &idx), ["alfa", "Beta", "Gamma", "Zeta"]);
        let idx = filter_sort(&l, "", None, None, SortField::Title, SortDirection::Desc);
        assert_eq!(titles(&l, &idx), ["Zeta", "Gamma", "Beta", "alfa"]);
    }

    #[test]
    fn ordena_por_duracion() {
        let l = library();
        let idx = filter_sort(&l, "", None, None, SortField::Duration, SortDirection::Asc);
        assert_eq!(titles(&l, &idx), ["Gamma", "alfa", "Beta", "Zeta"]);
    }

    #[test]
    fn filtra_por_artista() {
        let l = library();
        let idx = filter_sort(&l, "", Some("Metallica"), None, SortField::Title, SortDirection::Asc);
        assert_eq!(titles(&l, &idx), ["Beta", "Zeta"]);
    }

    #[test]
    fn la_busqueda_ordena_por_relevancia_e_ignora_el_orden_de_columna() {
        let l = vec![
            song("Otra", "X", "Black Album", 1, None),
            song("Black Dog", "Y", "Z", 1, None),
        ];
        let idx = filter_sort(&l, "black", None, None, SortField::Title, SortDirection::Desc);
        assert_eq!(titles(&l, &idx), ["Black Dog", "Otra"]);
    }

    #[test]
    fn la_busqueda_exige_que_todos_los_terminos_coincidan() {
        let l = library();
        assert_eq!(filter_sort(&l, "metallica load", None, None, SortField::Title, SortDirection::Asc).len(), 1);
        assert!(filter_sort(&l, "metallica nada", None, None, SortField::Title, SortDirection::Asc).is_empty());
    }

    #[test]
    fn agrupa_artistas_con_conteo_portada_y_desconocido() {
        let l = library();
        let groups = group_artists(&l, "", SortDirection::Asc);
        let names: Vec<&str> = groups.iter().map(|g| g.artist.as_str()).collect();
        assert_eq!(names, ["AC/DC", "Artista Desconocido", "Metallica"]);
        let met = groups.iter().find(|g| g.artist == "Metallica").unwrap();
        assert_eq!(met.count, 2);
        assert_eq!(met.representative_cover.as_deref(), Some("/c/met.jpg"));
    }

    #[test]
    fn agrupa_en_orden_descendente_y_respeta_la_busqueda() {
        let l = library();
        let groups = group_artists(&l, "", SortDirection::Desc);
        assert_eq!(groups[0].artist, "Metallica");
        let filtered = group_artists(&l, "black", SortDirection::Asc);
        let names: Vec<&str> = filtered.iter().map(|g| g.artist.as_str()).collect();
        assert_eq!(names, ["AC/DC", "Metallica"]);
    }

    #[test]
    fn no_recalcula_si_las_entradas_no_cambian() {
        let l = library();
        let mut view = LibraryView::default();
        let a = view.songs(&l, 1, "", None, None, SortField::Title, SortDirection::Asc);
        let b = view.songs(&l, 1, "", None, None, SortField::Title, SortDirection::Asc);
        assert_eq!(view.recomputes, 1);
        assert!(Arc::ptr_eq(&a, &b));
    }

    #[test]
    fn recalcula_cuando_cambia_cualquier_entrada() {
        let l = library();
        let mut view = LibraryView::default();
        view.songs(&l, 1, "", None, None, SortField::Title, SortDirection::Asc);
        view.songs(&l, 1, "met", None, None, SortField::Title, SortDirection::Asc);
        view.songs(&l, 1, "met", Some("Metallica"), None, SortField::Title, SortDirection::Asc);
        view.songs(&l, 1, "met", Some("Metallica"), None, SortField::Album, SortDirection::Asc);
        view.songs(&l, 1, "met", Some("Metallica"), None, SortField::Album, SortDirection::Desc);
        view.songs(&l, 2, "met", Some("Metallica"), None, SortField::Album, SortDirection::Desc);
        assert_eq!(view.recomputes, 6);
    }

    #[test]
    fn los_grupos_tambien_se_cachean_e_invalidan() {
        let l = library();
        let mut view = LibraryView::default();
        let a = view.groups(&l, 1, "", SortDirection::Asc);
        let b = view.groups(&l, 1, "", SortDirection::Asc);
        assert!(Arc::ptr_eq(&a, &b));
        view.groups(&l, 1, "", SortDirection::Desc);
        view.groups(&l, 2, "", SortDirection::Desc);
        assert_eq!(view.recomputes, 3);
    }

    // ── álbumes ────────────────────────────────────────────────────────────
    #[test]
    fn filtra_por_album() {
        let l = library();
        let idx = filter_sort(&l, "", Some("Metallica"), Some("Load"), SortField::Title, SortDirection::Asc);
        assert_eq!(titles(&l, &idx), ["Beta"]);
        let todos = filter_sort(&l, "", None, Some("Black"), SortField::Title, SortDirection::Asc);
        assert_eq!(titles(&l, &todos), ["Zeta"]);
    }

    #[test]
    fn agrupa_albumes_por_nombre_y_artista_con_duracion_y_portada() {
        let mut l = library();
        l.push(song("Delta", "Metallica", "Load", 50, None));
        l.push(song("Eps", "Otro", "Load", 10, Some("/c/otro.jpg")));
        let albums = group_albums(&l, "", SortDirection::Asc);
        let names: Vec<(&str, &str)> = albums.iter().map(|a| (a.album.as_str(), a.artist.as_str())).collect();
        assert_eq!(
            names,
            [("Back in Black", "AC/DC"), ("Black", "Metallica"), ("Load", "Metallica"), ("Load", "Otro"), ("Sin album", "")]
        );
        let load = &albums[2];
        assert_eq!((load.count, load.total_secs), (2, 300));
        assert_eq!(load.cover.as_deref(), Some("/c/met.jpg"));
    }

    #[test]
    fn los_albumes_respetan_busqueda_y_orden_descendente() {
        let l = library();
        let desc = group_albums(&l, "", SortDirection::Desc);
        assert_eq!(desc[0].album, "Sin album");
        let filtered = group_albums(&l, "load", SortDirection::Asc);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].album, "Load");
    }

    #[test]
    fn los_albumes_tambien_se_cachean() {
        let l = library();
        let mut view = LibraryView::default();
        let a = view.albums(&l, 1, "", SortDirection::Asc);
        let b = view.albums(&l, 1, "", SortDirection::Asc);
        assert!(Arc::ptr_eq(&a, &b));
        assert_eq!(view.recomputes, 1);
    }

    #[test]
    fn el_artista_desconocido_filtra_las_canciones_sin_artista() {
        let l = library();
        let idx = filter_sort(&l, "", Some(UNKNOWN_ARTIST), None, SortField::Title, SortDirection::Asc);
        assert_eq!(titles(&l, &idx), ["Gamma"]);
    }
}
