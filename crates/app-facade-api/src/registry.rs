#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationKind {
    Request,
    Event,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Authority {
    Cloud,
    Hub,
    Host,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Operation {
    pub id: &'static str,
    pub target: &'static str,
    pub kind: OperationKind,
    pub authority: Authority,
}

pub const SHARED_OPERATIONS: &[Operation] = &[
    Operation {
        id: "plugin.installed.list",
        target: crate::plugin::INSTALLED_LIST_TARGET,
        kind: OperationKind::Request,
        authority: Authority::Hub,
    },
    Operation {
        id: "plugin.settings.changed",
        target: crate::plugin::SETTINGS_CHANGED_TARGET,
        kind: OperationKind::Event,
        authority: Authority::Hub,
    },
    Operation {
        id: "runtime.status.list",
        target: crate::runtime::STATUS_LIST_TARGET,
        kind: OperationKind::Request,
        authority: Authority::Host,
    },
    Operation {
        id: "runtime.status.changed",
        target: crate::runtime::STATUS_CHANGED_TARGET,
        kind: OperationKind::Event,
        authority: Authority::Host,
    },
];

pub fn operation_by_target(target: &str) -> Option<&'static Operation> {
    SHARED_OPERATIONS
        .iter()
        .find(|operation| operation.target == target)
}
