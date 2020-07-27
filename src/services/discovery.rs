use serde::{Deserialize, Serialize};

/// Agent basic information, sent to host by Discovery regularly.
/// While server response is not expected.
#[derive(Debug, Serialize, Deserialize)]
pub struct AgentBasic {
    /// User set agent unique name.
    pub name: String,
    /// Agent NIC address.
    pub local_addr: String,
}
