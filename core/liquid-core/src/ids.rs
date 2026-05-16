use serde::{Deserialize, Serialize};
use uuid::Uuid;

macro_rules! uuid_newtype {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(pub Uuid);

        impl $name {
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

uuid_newtype!(
    WorkspaceId,
    "Identifies a workspace within a Liquid installation."
);
uuid_newtype!(
    AppInstanceId,
    "Identifies an installed app within a workspace."
);
uuid_newtype!(
    ComponentId,
    "Identifies a component within an app instance."
);
uuid_newtype!(PageId, "Identifies a page within a workspace.");
uuid_newtype!(RoleId, "Identifies an RBAC role within a workspace.");
uuid_newtype!(OperationId, "Identifies a Jujutsu operation log entry.");
uuid_newtype!(CommitId, "Identifies a Jujutsu commit.");

/// A principal: either a human user or an autonomous agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "lowercase")]
pub enum PrincipalId {
    User(Uuid),
    Agent(Uuid),
}

impl PrincipalId {
    pub fn new_user() -> Self {
        Self::User(Uuid::new_v4())
    }

    pub fn new_agent() -> Self {
        Self::Agent(Uuid::new_v4())
    }

    pub fn is_agent(&self) -> bool {
        matches!(self, Self::Agent(_))
    }
}

impl std::fmt::Display for PrincipalId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::User(id) => write!(f, "user:{id}"),
            Self::Agent(id) => write!(f, "agent:{id}"),
        }
    }
}

/// Parse a `PrincipalId` from its wire form. Round-trips with
/// [`std::fmt::Display`] — `"user:<uuid>"` ↔ `PrincipalId::User(_)`
/// and `"agent:<uuid>"` ↔ `PrincipalId::Agent(_)`. Also accepts the
/// short forms `"u:<uuid>"` / `"a:<uuid>"` produced by
/// `liquid_auth::token` (token-payload encoding) and the
/// `liquid audit list` NDJSON emit, so a
/// `liquid audit list --principal "$(jq -r .principal)"` round-trips
/// regardless of which layer produced the original string.
impl std::str::FromStr for PrincipalId {
    type Err = crate::LiquidError;

    fn from_str(s: &str) -> crate::Result<Self> {
        let (kind, id) = s.split_once(':').ok_or_else(|| {
            crate::LiquidError::InvalidInput(format!("principal id missing prefix: {s}"))
        })?;
        let uuid = Uuid::parse_str(id).map_err(|e| {
            crate::LiquidError::InvalidInput(format!("principal id not a uuid: {s}: {e}"))
        })?;
        match kind {
            "u" | "user" => Ok(Self::User(uuid)),
            "a" | "agent" => Ok(Self::Agent(uuid)),
            other => Err(crate::LiquidError::InvalidInput(format!(
                "principal kind not recognised: {other} (one of u, user, a, agent)"
            ))),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod principal_id_parse_tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn round_trips_user_long_form() {
        let p = PrincipalId::new_user();
        assert_eq!(PrincipalId::from_str(&p.to_string()).unwrap(), p);
    }

    #[test]
    fn round_trips_agent_long_form() {
        let p = PrincipalId::new_agent();
        assert_eq!(PrincipalId::from_str(&p.to_string()).unwrap(), p);
    }

    #[test]
    fn accepts_user_short_form() {
        let id = Uuid::new_v4();
        let parsed = PrincipalId::from_str(&format!("u:{id}")).unwrap();
        assert_eq!(parsed, PrincipalId::User(id));
    }

    #[test]
    fn accepts_agent_short_form() {
        let id = Uuid::new_v4();
        let parsed = PrincipalId::from_str(&format!("a:{id}")).unwrap();
        assert_eq!(parsed, PrincipalId::Agent(id));
    }

    #[test]
    fn rejects_missing_colon() {
        let err = PrincipalId::from_str("no-prefix").unwrap_err();
        assert!(matches!(err, crate::LiquidError::InvalidInput(_)));
    }

    #[test]
    fn rejects_unknown_kind() {
        let err = PrincipalId::from_str("bot:00000000-0000-0000-0000-000000000000").unwrap_err();
        assert!(matches!(err, crate::LiquidError::InvalidInput(_)));
    }

    #[test]
    fn rejects_bad_uuid() {
        let err = PrincipalId::from_str("user:not-a-uuid").unwrap_err();
        assert!(matches!(err, crate::LiquidError::InvalidInput(_)));
    }
}
