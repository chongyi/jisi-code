use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

macro_rules! define_id_type {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $name(Uuid);

        impl $name {
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }

            pub fn from_uuid(value: Uuid) -> Self {
                Self(value)
            }

            pub fn into_inner(self) -> Uuid {
                self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }

        impl FromStr for $name {
            type Err = uuid::Error;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Uuid::parse_str(s).map(Self)
            }
        }

        impl From<Uuid> for $name {
            fn from(value: Uuid) -> Self {
                Self::from_uuid(value)
            }
        }

        impl From<$name> for Uuid {
            fn from(value: $name) -> Self {
                value.into_inner()
            }
        }
    };
}

define_id_type!(UserId);
define_id_type!(ProblemId);
define_id_type!(SubmissionId);

#[cfg(test)]
mod tests {
    use super::UserId;

    #[test]
    fn user_id_can_roundtrip_from_string() {
        let id = UserId::new();
        let parsed: UserId = id
            .to_string()
            .parse()
            .expect("generated user id should be valid");

        assert_eq!(id, parsed);
    }
}
