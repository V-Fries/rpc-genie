use std::fmt::Display;

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RpcRequestId(pub u64);

impl Display for RpcRequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u64> for RpcRequestId {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

#[cfg(test)]
impl PartialEq<u64> for RpcRequestId {
    fn eq(&self, other: &u64) -> bool {
        self.0 == *other
    }
}
