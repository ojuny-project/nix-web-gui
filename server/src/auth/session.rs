use std::time::{Duration, Instant};

use rand::RngCore;
use std::collections::HashMap;
use std::sync::RwLock;

#[derive(Clone, Debug)]
pub struct Session {
    pub username: String,
    pub csrf: String,
    pub expires: Instant,
}

#[derive(Default)]
pub struct SessionStore {
    inner: RwLock<HashMap<String, Session>>,
}

impl SessionStore {
    pub fn create(&self, username: &str, ttl: Duration) -> (String, String) {
        let id = random_token();
        let csrf = random_token();
        let session = Session {
            username: username.to_string(),
            csrf: csrf.clone(),
            expires: Instant::now() + ttl,
        };
        self.inner
            .write()
            .expect("session store poisoned")
            .insert(id.clone(), session);
        (id, csrf)
    }

    pub fn get(&self, id: &str) -> Option<Session> {
        let session = self
            .inner
            .read()
            .expect("session store poisoned")
            .get(id)
            .cloned()?;
        if session.expires <= Instant::now() {
            self.remove(id);
            return None;
        }
        Some(session)
    }

    pub fn remove(&self, id: &str) {
        self.inner
            .write()
            .expect("session store poisoned")
            .remove(id);
    }

    pub fn validate_csrf(&self, id: &str, token: &str) -> bool {
        self.get(id)
            .map(|session| session.csrf == token)
            .unwrap_or(false)
    }

    pub fn purge_expired(&self) {
        let now = Instant::now();
        self.inner
            .write()
            .expect("session store poisoned")
            .retain(|_, session| session.expires > now);
    }
}

fn random_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_and_removes_a_session() {
        let store = SessionStore::default();
        let (id, csrf) = store.create("den", Duration::from_secs(60));
        assert_eq!(store.get(&id).unwrap().username, "den");
        assert!(store.validate_csrf(&id, &csrf));
        store.remove(&id);
        assert!(store.get(&id).is_none());
    }

    #[test]
    fn rejects_expired_sessions() {
        let store = SessionStore::default();
        let (id, _) = store.create("den", Duration::from_secs(0));
        std::thread::sleep(Duration::from_millis(1));
        assert!(store.get(&id).is_none());
    }
}
