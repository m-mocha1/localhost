use rand::{Rng, distributions::Alphanumeric};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Session {
    pub id: String,
    pub data: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct SessionManager {
    sessions: HashMap<String, Session>,
}

impl SessionManager {
    pub fn new() -> Self {
        SessionManager {
            sessions: HashMap::new(),
        }
    }

    /// gen session_id randomly
    fn generate_session_id() -> String {
        rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(32)
            .map(char::from)
            .collect()
    }
    /// get or create a session based on the cookie header
    pub fn get_or_create_session(&mut self, cookie_header: Option<&str>) -> Session {
        if let Some(cookie_header) = cookie_header {
            if let Some(session_id) = Self::extract_session_id(cookie_header) {
                println!("🔑 Found session_id in cookies: {}", session_id);
                if let Some(session) = self.sessions.get(&session_id) {
                    println!("✅ Reusing existing session: {}", session_id);
                    return session.clone();
                } else {
                    println!("❌ Session ID not found in session store");
                }
            } else {
                println!("❌ No session_id found in cookies");
            }
        } else {
            println!("📭 No Cookie header received");
        }

        // create a new session if no valid session_id is found
        let new_id = Self::generate_session_id();
        let new_session = Session {
            id: new_id.clone(),
            data: HashMap::new(),
        };

        self.sessions.insert(new_id.clone(), new_session.clone());
        new_session
    }

    /// extract session_id from the cookie header to use in get_or_create_session
    fn extract_session_id(cookie_header: &str) -> Option<String> {
        for cookie in cookie_header.split(';') {
            let cookie = cookie.trim();
            if cookie.starts_with("session_id=") {
                return Some(cookie.trim_start_matches("session_id=").to_string());
            }
        }
        None
    }
}
