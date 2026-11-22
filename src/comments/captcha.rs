use std::collections::HashMap;
use std::sync::Mutex;

use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

pub struct CaptchaChallenge {
    pub id: String,
    pub question: String,
    pub svg: String,
}

struct CaptchaEntry {
    answer: String,
    expires_at: DateTime<Utc>,
}

pub struct CaptchaStore {
    inner: Mutex<HashMap<String, CaptchaEntry>>,
    ttl: Duration,
}

impl CaptchaStore {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
            ttl: Duration::minutes(15),
        }
    }

    pub fn generate(&self) -> CaptchaChallenge {
        let (question, answer) = math_challenge();
        let svg = captcha_svg(&question);
        let id = Uuid::new_v4().to_string();

        let mut store = self.inner.lock().unwrap();
        cleanup(&mut store);
        store.insert(
            id.clone(),
            CaptchaEntry {
                answer,
                expires_at: Utc::now() + self.ttl,
            },
        );

        CaptchaChallenge {
            id,
            question,
            svg,
        }
    }

    pub fn verify(&self, id: &str, answer: &str) -> bool {
        let mut store = self.inner.lock().unwrap();
        cleanup(&mut store);
        match store.remove(id) {
            Some(entry) if Utc::now() <= entry.expires_at => {
                normalize_answer(answer) == entry.answer
            }
            _ => false,
        }
    }
}

impl Default for CaptchaStore {
    fn default() -> Self {
        Self::new()
    }
}

fn math_challenge() -> (String, String) {
    let bytes = Uuid::new_v4().into_bytes();
    let a = (bytes[0] % 9) + 1;
    let b = (bytes[1] % 9) + 1;
    if bytes[2] % 2 == 0 {
        (
            format!("What is {} + {}?", a, b),
            (a as u16 + b as u16).to_string(),
        )
    } else {
        let (hi, lo) = if a >= b { (a, b) } else { (b, a) };
        (
            format!("What is {} - {}?", hi, lo),
            (hi - lo).to_string(),
        )
    }
}

fn normalize_answer(answer: &str) -> String {
    answer.trim().to_string()
}

fn captcha_svg(question: &str) -> String {
    let rotation = (Uuid::new_v4().as_bytes()[0] % 7) as i32 - 3;
    let noise_x1 = 10 + (Uuid::new_v4().as_bytes()[0] % 40);
    let noise_y1 = 8 + (Uuid::new_v4().as_bytes()[0] % 20);
    let noise_x2 = 90 + (Uuid::new_v4().as_bytes()[0] % 50);
    let noise_y2 = 20 + (Uuid::new_v4().as_bytes()[0] % 20);

    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="220" height="48" viewBox="0 0 220 48" role="img" aria-label="{label}">
  <rect width="220" height="48" fill="#f4f4f4" stroke="#888888"/>
  <line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="#cccccc" stroke-width="2"/>
  <text x="110" y="30" text-anchor="middle" font-size="16" font-family="serif" fill="#333333" transform="rotate({rot} 110 24)">{text}</text>
</svg>"##,
        label = crate::comments::escape_html(question),
        text = crate::comments::escape_html(question),
        rot = rotation,
        x1 = noise_x1,
        y1 = noise_y1,
        x2 = noise_x2,
        y2 = noise_y2,
    )
}

fn cleanup(store: &mut HashMap<String, CaptchaEntry>) {
    let now = Utc::now();
    store.retain(|_, entry| entry.expires_at > now);
    if store.len() > 10_000 {
        store.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_captcha_accepts_correct_answer() {
        let store = CaptchaStore::new();
        let challenge = store.generate();
        let answer = {
            let inner = store.inner.lock().unwrap();
            inner.get(&challenge.id).unwrap().answer.clone()
        };
        assert!(store.verify(&challenge.id, &answer));
    }

    #[test]
    fn test_captcha_trims_answer() {
        let store = CaptchaStore::new();
        let challenge = store.generate();
        let answer = {
            let inner = store.inner.lock().unwrap();
            inner.get(&challenge.id).unwrap().answer.clone()
        };
        assert!(store.verify(&challenge.id, &format!("  {}  ", answer)));
    }

    #[test]
    fn test_captcha_rejects_wrong_answer() {
        let store = CaptchaStore::new();
        let challenge = store.generate();
        assert!(!store.verify(&challenge.id, "999"));
    }

    #[test]
    fn test_captcha_is_single_use() {
        let store = CaptchaStore::new();
        let challenge = store.generate();
        let answer = {
            let inner = store.inner.lock().unwrap();
            inner.get(&challenge.id).unwrap().answer.clone()
        };
        assert!(store.verify(&challenge.id, &answer));
        assert!(!store.verify(&challenge.id, &answer));
    }

    #[test]
    fn test_captcha_unknown_id_fails() {
        let store = CaptchaStore::new();
        assert!(!store.verify("missing", "1"));
    }

    #[test]
    fn test_math_challenge_format() {
        for _ in 0..20 {
            let (question, answer) = math_challenge();
            assert!(question.starts_with("What is "));
            let n: i32 = answer.parse().unwrap();
            assert!((0..=18).contains(&n));
        }
    }

    #[test]
    fn test_svg_contains_question() {
        let svg = captcha_svg("What is 2 + 2?");
        assert!(svg.contains("What is 2 + 2?"));
        assert!(svg.contains("<svg"));
    }
}
