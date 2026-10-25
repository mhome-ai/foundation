// Generated from manifest/operation-timeouts.v1.json. DO NOT EDIT.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    CoreRequest,
    HarnessSessionOpen,
    ScriptValidation,
    ScriptSave,
    CoreBridgeRequest,
    CoreBridgeInbound,
    NodeRequest,
    LionBridgeRequest,
    LlmCloudCall,
}

impl Operation {
    pub const fn id(self) -> &'static str {
        match self {
            Self::CoreRequest => "coreRequest",
            Self::HarnessSessionOpen => "harnessSessionOpen",
            Self::ScriptValidation => "scriptValidation",
            Self::ScriptSave => "scriptSave",
            Self::CoreBridgeRequest => "coreBridgeRequest",
            Self::CoreBridgeInbound => "coreBridgeInbound",
            Self::NodeRequest => "nodeRequest",
            Self::LionBridgeRequest => "lionBridgeRequest",
            Self::LlmCloudCall => "llmCloudCall",
        }
    }

    pub const fn timeout(self) -> Duration {
        Duration::from_millis(match self {
            Self::CoreRequest => 60000,
            Self::HarnessSessionOpen => 150000,
            Self::ScriptValidation => 205000,
            Self::ScriptSave => 420000,
            Self::CoreBridgeRequest => 35000,
            Self::CoreBridgeInbound => 55000,
            Self::NodeRequest => 60000,
            Self::LionBridgeRequest => 60000,
            Self::LlmCloudCall => 180000,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitProfile {
    HostIpc,
    CoreBridge,
    Client,
    LionForward,
}

impl WaitProfile {
    pub const fn grace(self, operation: Operation) -> Duration {
        Duration::from_millis(match (self, operation) {
            (Self::HostIpc, _) => 5000,
            (Self::CoreBridge, _) => 10000,
            (Self::Client, _) => 35000,
            (Self::LionForward, Operation::ScriptValidation) => 25000,
            (Self::LionForward, Operation::ScriptSave) => 30000,
            (Self::LionForward, _) => 0,
        })
    }
}

const FORWARD_PREFIXES: &[&str] = &[
    "/invoke",
    "/dispatch/agent",
    "/dispatch",
];

const STORED_SCRIPT_SAVE: &[&str] = &[
    "/automation/snapshot/rollback",
    "/app/automation/snapshot/rollback",
    "/automation/overview/save",
    "/automation/copy",
    "/automation/enroll",
];

const SOURCE_SCRIPT_SAVE: &[&str] = &[
    "/automation/save",
    "/app/automation/save",
];

const SOURCE_SCRIPT_VALIDATION: &[&str] = &[
    "/app/automation/verify",
    "/bridge/automation/parse",
    "/bridge/automation/format",
    "/bridge/automation/verify",
    "/automation/format",
    "/bridge/automation/compile",
    "/bridge/agent/automation/compile",
];

const ORDINARY_OVERRIDES: &[(&str, Operation)] = &[
    ("/harness/test/session/open", Operation::HarnessSessionOpen),
];
