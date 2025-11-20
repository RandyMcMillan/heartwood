use serde::Deserialize;

/// Represent the status of a step in a roadmap.
///
/// The unknown status allows the user to not specify the status, and
/// the roadmap to infer it from the structure of the graph. For
/// example, a step is inferred to be blocked if any of it
/// dependencies are not finished.

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Unknown,
    Goal,
    Finished,
    Ready,
    Next,
    Blocked,
}

impl Default for Status {
    fn default() -> Self {
        Self::Unknown
    }
}

impl Status {
    pub fn from_text(text: &str) -> Option<Status> {
        match text {
            "" => Some(Status::Unknown),
            "goal" => Some(Status::Goal),
            "finished" => Some(Status::Finished),
            "ready" => Some(Status::Ready),
            "next" => Some(Status::Next),
            "blocked" => Some(Status::Blocked),
            _ => None,
        }
    }
}

#[cfg(test)]
mod test {
    use super::Status;

    #[test]
    fn happy_from_text() {
        assert_eq!(Status::from_text("").unwrap(), Status::Unknown);
        assert_eq!(Status::from_text("goal").unwrap(), Status::Goal);
        assert_eq!(Status::from_text("finished").unwrap(), Status::Finished);
        assert_eq!(Status::from_text("ready").unwrap(), Status::Ready);
        assert_eq!(Status::from_text("next").unwrap(), Status::Next);
        assert_eq!(Status::from_text("blocked").unwrap(), Status::Blocked);
    }

    #[test]
    fn sad_from_text() {
        let x = Status::from_text("x");
        assert_eq!(x, None);
    }
}
