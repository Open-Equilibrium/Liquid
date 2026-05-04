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
