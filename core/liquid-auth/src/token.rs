use std::time::{SystemTime, UNIX_EPOCH};

use hmac::{Hmac, Mac};
use liquid_core::{LiquidError, PrincipalId, Result};
use sha2::Sha256;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

/// `principal_id . expires_unix . hmac_hex`.
///
/// `IMPLEMENTATION_PLAN.md` §9 originally specified
/// `principal . workspace . expires . hmac` but Phase-1 token semantics do
/// not bind a session to a specific workspace (a single token is presented
/// for any workspace the principal has bindings in — workspace authority is
/// determined by the `PermissionIndex`, not the token). Carrying the field
/// would invite the bug of misinterpreting it as authorisation. Documented
/// in §4.5.
pub(crate) struct TokenPayload {
    pub principal: PrincipalId,
    pub expires_unix: u64,
}

impl TokenPayload {
    fn payload_str(&self) -> String {
        format!("{}.{}", encode_principal(self.principal), self.expires_unix)
    }
}

pub(crate) fn build_token(payload: &TokenPayload, secret: &[u8]) -> Result<String> {
    let body = payload.payload_str();
    let signature = sign(&body, secret)?;
    Ok(format!("{body}.{}", hex::encode(signature)))
}

pub(crate) fn parse_and_verify_token(token: &str, secret: &[u8]) -> Result<PrincipalId> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err(LiquidError::Forbidden);
    }
    let principal = decode_principal(parts[0]).ok_or(LiquidError::Forbidden)?;
    let expires_unix: u64 = parts[1].parse().map_err(|_| LiquidError::Forbidden)?;
    let provided = hex::decode(parts[2]).map_err(|_| LiquidError::Forbidden)?;

    let body = format!("{}.{}", parts[0], parts[1]);
    let mut mac = HmacSha256::new_from_slice(secret)
        .map_err(|_| LiquidError::InvalidInput("HMAC key error".into()))?;
    mac.update(body.as_bytes());
    mac.verify_slice(&provided)
        .map_err(|_| LiquidError::Forbidden)?;

    if expires_unix < now_unix() {
        return Err(LiquidError::Forbidden);
    }
    Ok(principal)
}

fn sign(body: &str, secret: &[u8]) -> Result<Vec<u8>> {
    let mut mac = HmacSha256::new_from_slice(secret)
        .map_err(|_| LiquidError::InvalidInput("HMAC key error".into()))?;
    mac.update(body.as_bytes());
    Ok(mac.finalize().into_bytes().to_vec())
}

/// Encode a `PrincipalId` to a dot-free, URL-safe string. We use `u:UUID`
/// or `a:UUID` (colon, not dot, since dot is the field separator).
fn encode_principal(p: PrincipalId) -> String {
    match p {
        PrincipalId::User(id) => format!("u:{id}"),
        PrincipalId::Agent(id) => format!("a:{id}"),
    }
}

fn decode_principal(s: &str) -> Option<PrincipalId> {
    let (kind, id) = s.split_once(':')?;
    let uuid = Uuid::parse_str(id).ok()?;
    match kind {
        "u" => Some(PrincipalId::User(uuid)),
        "a" => Some(PrincipalId::Agent(uuid)),
        _ => None,
    }
}

pub(crate) fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
