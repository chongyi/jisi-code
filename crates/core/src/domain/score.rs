use super::DomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Score(u16);

impl Score {
    pub const MIN: u16 = 0;
    pub const MAX: u16 = 100;

    pub fn new(value: u16) -> Result<Self, DomainError> {
        if value <= Self::MAX {
            Ok(Self(value))
        } else {
            Err(DomainError::InvalidScore(value))
        }
    }

    pub fn value(self) -> u16 {
        self.0
    }
}

impl Default for Score {
    fn default() -> Self {
        Self(Self::MIN)
    }
}

impl TryFrom<u16> for Score {
    type Error = DomainError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<Score> for u16 {
    fn from(value: Score) -> Self {
        value.value()
    }
}

#[cfg(test)]
mod tests {
    use super::Score;

    #[test]
    fn valid_score_is_created() {
        let score = Score::new(100).expect("100 should be valid");

        assert_eq!(score.value(), 100);
    }

    #[test]
    fn invalid_score_is_rejected() {
        let err = Score::new(101).expect_err("101 should be rejected");

        assert_eq!(
            err.to_string(),
            "invalid score: 101. score must be in [0, 100]"
        );
    }
}
