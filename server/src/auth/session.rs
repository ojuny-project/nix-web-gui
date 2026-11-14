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
    pub fn create(&self, username: &str, ttl: Duration, max_sessions: usize) -> (String, String) {
        let id = random_token();
        let csrf = random_token();
        let session = Session {
            username: username.to_string(),
            csrf: csrf.clone(),
            expires: Instant::now() + ttl,
        };
        let mut sessions = self.inner.write().expect("session store poisoned");
        sessions.insert(id.clone(), session);
        let limit = max_sessions.max(1);
        while sessions.len() > limit {
            let oldest = sessions
                .iter()
                .min_by_key(|(_, session)| session.expires)
                .map(|(id, _)| id.clone());
            match oldest {
                Some(id) => {
                    sessions.remove(&id);
                }
                None => break,
            }
        }
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

    pub fn ids(&self) -> std::collections::HashSet<String> {
        self.inner
            .read()
            .expect("session store poisoned")
            .keys()
            .cloned()
            .collect()
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
        let (id, csrf) = store.create("den", Duration::from_secs(60), 8);
        assert_eq!(store.get(&id).unwrap().username, "den");
        assert!(store.validate_csrf(&id, &csrf));
        store.remove(&id);
        assert!(store.get(&id).is_none());
    }

    #[test]
    fn rejects_expired_sessions() {
        let store = SessionStore::default();
        let (id, _) = store.create("den", Duration::from_secs(0), 8);
        std::thread::sleep(Duration::from_millis(1));
        assert!(store.get(&id).is_none());
    }

    #[test]
    fn caps_the_number_of_active_sessions() {
        let store = SessionStore::default();
        let first = store.create("alice", Duration::from_secs(60), 2).0;
        let second = store.create("alice", Duration::from_secs(61), 2).0;
        let third = store.create("alice", Duration::from_secs(62), 2).0;
        assert!(store.get(&first).is_none());
        assert!(store.get(&second).is_some());
        assert!(store.get(&third).is_some());
    }
}
