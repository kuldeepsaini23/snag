use crate::model::{AppState, Status};
use std::path::Path;

/// Missing or unreadable state starts fresh. Items that were running when the app
/// stopped come back paused.
pub fn load(path: &Path) -> AppState {
    let Some(mut state) = std::fs::read(path).ok().and_then(|b| serde_json::from_slice::<AppState>(&b).ok()) else {
        return AppState::default();
    };
    for item in &mut state.items {
        if item.status == Status::Running {
            item.status = Status::Paused;
        }
        item.speed_bps = 0;
    }
    state
}

/// Atomic: temp file, then rename.
pub fn save(path: &Path, state: &AppState) -> std::io::Result<()> {
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(state).map_err(std::io::Error::other)?)?;
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::category::Category;
    use crate::model::{Item, ItemId};

    fn sample() -> AppState {
        let mut s = AppState { next_id: 3, ..Default::default() };
        s.items.push(Item {
            id: ItemId(2),
            url: "http://x/a.zip".into(),
            name: "a.zip".into(),
            category: Category::Archive,
            status: Status::Running,
            dest: Some("C:\\dl\\a.zip".into()),
            downloaded: 10,
            total: Some(100),
            speed_bps: 5,
            queue: 0,
            added: 1,
            kind: Default::default(),
        });
        s
    }

    #[test]
    fn store_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("state.json");
        let mut s = sample();
        s.items[0].status = Status::Done;
        s.items[0].speed_bps = 0;
        save(&p, &s).unwrap();
        assert_eq!(load(&p), s);
    }

    #[test]
    fn store_load_turns_running_into_paused() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("state.json");
        save(&p, &sample()).unwrap();
        let loaded = load(&p);
        assert_eq!(loaded.items[0].status, Status::Paused);
        assert_eq!(loaded.items[0].speed_bps, 0);
    }

    #[test]
    fn old_state_without_kind_loads_as_http() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("state.json");
        let old = r#"{"next_id":2,"items":[{"id":1,"url":"http://x/a.zip","name":"a.zip","category":"Archive",
            "status":"Paused","dest":null,"downloaded":0,"total":null,"queue":0,"added":0}]}"#;
        std::fs::write(&p, old).unwrap();
        let loaded = load(&p);
        assert_eq!(loaded.items.len(), 1);
        assert_eq!(loaded.items[0].kind, crate::model::Kind::Http);
    }

    #[test]
    fn media_kind_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("state.json");
        let mut s = sample();
        s.items[0].kind = crate::model::Kind::Media(rdm_media::MediaFormat::Video { max_height: 720 });
        s.items[0].status = Status::Paused;
        s.items[0].speed_bps = 0;
        save(&p, &s).unwrap();
        assert_eq!(load(&p), s);
    }

    #[test]
    fn store_missing_or_corrupt_gives_default() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("state.json");
        assert_eq!(load(&p), AppState::default());
        std::fs::write(&p, b"{oops").unwrap();
        assert_eq!(load(&p), AppState::default());
    }
}
