//! Canonical Interaction Flow definition and Core-to-Node wire contract.
//!
//! Definitions describe presentation, typed inputs, transitions, and logical
//! operations. They never select a Node instance or transport target. MeowCore
//! binds an executor when it creates a session and owns all routing decisions.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

pub const PROTOCOL_VERSION: u32 = 1;
pub const NODE_PROTOCOL_SCHEMA: &str =
    include_str!("../schema/interaction-flow-node.v1.schema.json");
pub const MAX_STEPS: usize = 64;
pub const MAX_RESOLVERS: usize = 128;
pub const MAX_FIELDS_PER_STEP: usize = 64;
pub const MAX_PRESENTATION_BYTES: usize = 64 * 1024;
pub const MAX_VALUE_REF_PATH: usize = 32;
pub const MAX_HANDLER_ARGS: usize = 128;
pub const MAX_OPTIONS: usize = 1024;
pub const MAX_TITLE_CHARS: usize = 1024;
pub const MAX_DESCRIPTION_CHARS: usize = 16 * 1024;
pub const MAX_CONTENT_CHARS: usize = 64 * 1024;
pub const MAX_LABEL_CHARS: usize = 1024;
pub const MAX_UNAVAILABLE_TEXT_CHARS: usize = 4096;
pub const MAX_MIN_ROWS: u32 = 100;

pub fn definition_target(node_type: &str) -> String {
    format!("/{node_type}/interaction-flow/definition")
}

pub fn execute_target(node_type: &str) -> String {
    format!("/{node_type}/interaction-flow/execute")
}

pub fn close_target(node_type: &str) -> String {
    format!("/{node_type}/interaction-flow/close")
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FlowValueRef {
    pub root: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub path: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum FlowArgument {
    Value { value: Value },
    Ref { value_ref: FlowValueRef },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FlowRenderer {
    TypedForm,
    /// Trusted MeowCore-only compatibility renderer used by the Custom
    /// Connect adapter. It is deliberately not part of the serialized Node
    /// protocol.
    #[serde(skip)]
    CustomConnectMdx,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FlowPresentation {
    pub renderer: FlowRenderer,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FlowForm {
    #[serde(default)]
    pub fields: Vec<FlowFormField>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FlowFieldValueType {
    String,
    Boolean,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FlowFieldControl {
    Text,
    Password,
    LongText,
    Select,
    Boolean,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FlowFormField {
    pub var: String,
    pub value_type: FlowFieldValueType,
    pub control: FlowFieldControl,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub repeat: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_value: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_value_ref: Option<FlowValueRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options_ref: Option<FlowValueRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub option_unavailable_text: Option<String>,
    #[serde(default)]
    pub read_only: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_rows: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum FlowTransition {
    Step { step_id: String },
    Dynamic { value_ref: FlowValueRef },
    Finish,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FlowHandler {
    pub operation: String,
    #[serde(default)]
    pub args: Vec<FlowArgument>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FlowValueResolver {
    pub output_var: String,
    pub handler: FlowHandler,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExternalAuthCompletionMode {
    Callback,
    Poll,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum FlowStepContent {
    Form {
        presentation: FlowPresentation,
        form: FlowForm,
        #[serde(default)]
        render_refs: Vec<FlowValueRef>,
    },
    ExternalAuth {
        presentation: FlowPresentation,
        authorization_url_ref: FlowValueRef,
        completion_mode: ExternalAuthCompletionMode,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        poll_after_ms: Option<u64>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InteractionFlowStep {
    pub id: String,
    pub content: FlowStepContent,
    pub transition: FlowTransition,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FlowLifecycleHandlers {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub complete: Option<FlowHandler>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cancel: Option<FlowHandler>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expire: Option<FlowHandler>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InteractionFlowDefinition {
    pub protocol_version: u32,
    pub flow_id: String,
    pub start_step_id: String,
    pub steps: HashMap<String, InteractionFlowStep>,
    #[serde(default)]
    pub resolvers: HashMap<String, FlowValueResolver>,
    #[serde(default)]
    pub lifecycle: FlowLifecycleHandlers,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowValidationError {
    message: String,
}

impl FlowValidationError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for FlowValidationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for FlowValidationError {}

impl InteractionFlowDefinition {
    pub fn validate(&self) -> Result<(), FlowValidationError> {
        if self.protocol_version != PROTOCOL_VERSION {
            return Err(FlowValidationError::new(
                "unsupported interaction-flow protocol version",
            ));
        }
        require_identifier("flow id", &self.flow_id)?;
        require_identifier("start step id", &self.start_step_id)?;
        if self.steps.is_empty() || self.steps.len() > MAX_STEPS {
            return Err(FlowValidationError::new(
                "interaction-flow step count is out of bounds",
            ));
        }
        if self.resolvers.len() > MAX_RESOLVERS {
            return Err(FlowValidationError::new(
                "interaction-flow resolver count is out of bounds",
            ));
        }
        if !self.steps.contains_key(&self.start_step_id) {
            return Err(FlowValidationError::new(
                "interaction-flow start step does not exist",
            ));
        }

        let mut inputs = HashSet::new();
        let mut input_shapes = HashMap::new();
        for (step_key, step) in &self.steps {
            require_identifier("step id", step_key)?;
            if step.id != *step_key {
                return Err(FlowValidationError::new(
                    "interaction-flow step key/id mismatch",
                ));
            }
            validate_presentation(step.presentation())?;
            if let FlowStepContent::Form { form, .. } = &step.content {
                if form.fields.len() > MAX_FIELDS_PER_STEP {
                    return Err(FlowValidationError::new(
                        "interaction-flow field count is out of bounds",
                    ));
                }
                let mut step_inputs = HashSet::new();
                for field in &form.fields {
                    require_variable("input variable", &field.var)?;
                    if is_builtin(&field.var) || !step_inputs.insert(field.var.as_str()) {
                        return Err(FlowValidationError::new(format!(
                            "duplicate or reserved interaction-flow input variable: {}",
                            field.var
                        )));
                    }
                    inputs.insert(field.var.as_str());
                    validate_field_shape(field)?;
                    if let Some((value_type, repeat)) = input_shapes.get(&field.var) {
                        if *value_type != field.value_type || *repeat != field.repeat {
                            return Err(FlowValidationError::new(format!(
                                "interaction-flow input changes value type or cardinality across steps: {}",
                                field.var
                            )));
                        }
                    } else {
                        input_shapes.insert(field.var.clone(), (field.value_type, field.repeat));
                    }
                }
            }
        }

        for (name, resolver) in &self.resolvers {
            require_variable("resolver output", name)?;
            if resolver.output_var != *name {
                return Err(FlowValidationError::new(
                    "interaction-flow resolver key/output mismatch",
                ));
            }
            if is_builtin(name) || inputs.contains(name.as_str()) {
                return Err(FlowValidationError::new(format!(
                    "duplicate or reserved interaction-flow resolver output: {name}"
                )));
            }
        }

        let known = |name: &str| {
            is_builtin(name) || inputs.contains(name) || self.resolvers.contains_key(name)
        };
        for step in self.steps.values() {
            match &step.transition {
                FlowTransition::Step { step_id } if !self.steps.contains_key(step_id) => {
                    return Err(FlowValidationError::new(format!(
                        "unknown interaction-flow step: {step_id}"
                    )));
                }
                FlowTransition::Dynamic { value_ref } => {
                    validate_ref(value_ref, &known)?;
                    if !self.resolvers.contains_key(&value_ref.root) {
                        return Err(FlowValidationError::new(
                            "dynamic interaction-flow transition must be resolver-backed",
                        ));
                    }
                }
                _ => {}
            }
            match &step.content {
                FlowStepContent::Form {
                    form, render_refs, ..
                } => {
                    for value_ref in render_refs {
                        validate_ref(value_ref, &known)?;
                    }
                    for field in &form.fields {
                        for value_ref in
                            [field.default_value_ref.as_ref(), field.options_ref.as_ref()]
                                .into_iter()
                                .flatten()
                        {
                            validate_ref(value_ref, &known)?;
                        }
                    }
                }
                FlowStepContent::ExternalAuth {
                    authorization_url_ref,
                    poll_after_ms,
                    completion_mode,
                    ..
                } => {
                    validate_ref(authorization_url_ref, &known)?;
                    if matches!(completion_mode, ExternalAuthCompletionMode::Poll)
                        && poll_after_ms.is_none_or(|value| !(250..=60_000).contains(&value))
                    {
                        return Err(FlowValidationError::new(
                            "polling external-auth steps require pollAfterMs between 250 and 60000",
                        ));
                    }
                }
            }
        }

        for resolver in self.resolvers.values() {
            validate_handler(&resolver.handler, &known)?;
        }
        for handler in [
            self.lifecycle.complete.as_ref(),
            self.lifecycle.cancel.as_ref(),
            self.lifecycle.expire.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            validate_handler(handler, &known)?;
        }
        validate_resolver_graph(&self.resolvers)
    }
}

impl InteractionFlowStep {
    pub fn presentation(&self) -> &FlowPresentation {
        match &self.content {
            FlowStepContent::Form { presentation, .. }
            | FlowStepContent::ExternalAuth { presentation, .. } => presentation,
        }
    }
}

fn validate_presentation(presentation: &FlowPresentation) -> Result<(), FlowValidationError> {
    if presentation
        .title
        .as_ref()
        .is_some_and(|value| value.chars().count() > MAX_TITLE_CHARS)
        || presentation
            .description
            .as_ref()
            .is_some_and(|value| value.chars().count() > MAX_DESCRIPTION_CHARS)
        || presentation
            .content
            .as_ref()
            .is_some_and(|value| value.chars().count() > MAX_CONTENT_CHARS)
    {
        return Err(FlowValidationError::new(
            "interaction-flow presentation field is too large",
        ));
    }
    let total = presentation.title.as_deref().unwrap_or_default().len()
        + presentation
            .description
            .as_deref()
            .unwrap_or_default()
            .len()
        + presentation.content.as_deref().unwrap_or_default().len();
    if total > MAX_PRESENTATION_BYTES {
        return Err(FlowValidationError::new(
            "interaction-flow presentation is too large",
        ));
    }
    if matches!(presentation.renderer, FlowRenderer::CustomConnectMdx)
        && presentation.content.as_deref().is_none_or(str::is_empty)
    {
        return Err(FlowValidationError::new(
            "custom-connect presentation requires content",
        ));
    }
    Ok(())
}

fn validate_field_shape(field: &FlowFormField) -> Result<(), FlowValidationError> {
    if field
        .label
        .as_ref()
        .is_some_and(|value| value.chars().count() > MAX_LABEL_CHARS)
        || field
            .option_unavailable_text
            .as_ref()
            .is_some_and(|value| value.chars().count() > MAX_UNAVAILABLE_TEXT_CHARS)
        || field
            .min_rows
            .is_some_and(|value| value == 0 || value > MAX_MIN_ROWS)
        || field
            .options
            .as_ref()
            .is_some_and(|options| options.len() > MAX_OPTIONS)
    {
        return Err(FlowValidationError::new(format!(
            "interaction-flow field metadata is out of bounds: {}",
            field.var
        )));
    }
    let compatible = matches!(
        (field.value_type, field.control),
        (FlowFieldValueType::String, FlowFieldControl::Text)
            | (FlowFieldValueType::String, FlowFieldControl::Password)
            | (FlowFieldValueType::String, FlowFieldControl::LongText)
            | (FlowFieldValueType::String, FlowFieldControl::Select)
            | (FlowFieldValueType::Boolean, FlowFieldControl::Boolean)
    );
    if !compatible {
        return Err(FlowValidationError::new(format!(
            "interaction-flow field has incompatible value type and control: {}",
            field.var
        )));
    }
    if field.repeat
        && !matches!(
            field.control,
            FlowFieldControl::Text | FlowFieldControl::Password | FlowFieldControl::LongText
        )
    {
        return Err(FlowValidationError::new(format!(
            "interaction-flow field control cannot repeat: {}",
            field.var
        )));
    }
    if field.default_value.is_some() && field.default_value_ref.is_some() {
        return Err(FlowValidationError::new(format!(
            "interaction-flow field has two default sources: {}",
            field.var
        )));
    }
    if matches!(field.control, FlowFieldControl::Password)
        && (field.default_value.is_some() || field.default_value_ref.is_some() || field.read_only)
    {
        return Err(FlowValidationError::new(format!(
            "interaction-flow password fields cannot have defaults or be read-only: {}",
            field.var
        )));
    }
    if field.options.is_some() && field.options_ref.is_some() {
        return Err(FlowValidationError::new(format!(
            "interaction-flow field has two option sources: {}",
            field.var
        )));
    }
    if matches!(field.control, FlowFieldControl::Select)
        && field.options.is_none()
        && field.options_ref.is_none()
    {
        return Err(FlowValidationError::new(format!(
            "interaction-flow select field requires options: {}",
            field.var
        )));
    }
    if !matches!(field.control, FlowFieldControl::Select)
        && (field.options.is_some()
            || field.options_ref.is_some()
            || field.option_unavailable_text.is_some())
    {
        return Err(FlowValidationError::new(format!(
            "interaction-flow options require a select control: {}",
            field.var
        )));
    }
    if field.min_rows.is_some() && !matches!(field.control, FlowFieldControl::LongText) {
        return Err(FlowValidationError::new(format!(
            "interaction-flow minRows requires a long-text control: {}",
            field.var
        )));
    }
    if let Some(default) = &field.default_value {
        validate_static_default(field, default)?;
        if let Some(options) = &field.options {
            let selected = if field.repeat {
                default.as_array().map(Vec::as_slice).unwrap_or_default()
            } else {
                std::slice::from_ref(default)
            };
            if selected.iter().any(|selected| {
                !options
                    .iter()
                    .any(|option| option_value(option) == Some(selected))
            }) {
                return Err(FlowValidationError::new(format!(
                    "interaction-flow default is not in static options: {}",
                    field.var
                )));
            }
        }
    }
    if let Some(options) = &field.options {
        for option in options {
            let value = option_value(option).ok_or_else(|| {
                FlowValidationError::new(format!(
                    "interaction-flow option has an invalid shape: {}",
                    field.var
                ))
            })?;
            validate_static_scalar(field, value, "option")?;
        }
    }
    Ok(())
}

fn validate_static_default(
    field: &FlowFormField,
    value: &Value,
) -> Result<(), FlowValidationError> {
    if field.repeat {
        let values = value.as_array().ok_or_else(|| {
            FlowValidationError::new(format!(
                "interaction-flow repeat default must be an array: {}",
                field.var
            ))
        })?;
        for value in values {
            validate_static_scalar(field, value, "default")?;
        }
    } else {
        validate_static_scalar(field, value, "default")?;
    }
    Ok(())
}

fn validate_static_scalar(
    field: &FlowFormField,
    value: &Value,
    kind: &str,
) -> Result<(), FlowValidationError> {
    let valid = match field.value_type {
        FlowFieldValueType::String => value.is_string(),
        FlowFieldValueType::Boolean => value.is_boolean(),
    };
    if !valid {
        return Err(FlowValidationError::new(format!(
            "interaction-flow {kind} has the wrong type: {}",
            field.var
        )));
    }
    Ok(())
}

fn option_value(option: &Value) -> Option<&Value> {
    let Value::Object(object) = option else {
        return Some(option);
    };
    if object.get("label").is_some_and(|label| !label.is_string()) {
        return None;
    }
    object.get("value")
}

fn validate_handler(
    handler: &FlowHandler,
    known: &impl Fn(&str) -> bool,
) -> Result<(), FlowValidationError> {
    require_identifier("handler operation", &handler.operation)?;
    if handler.args.len() > MAX_HANDLER_ARGS {
        return Err(FlowValidationError::new(
            "interaction-flow handler argument count is out of bounds",
        ));
    }
    for argument in &handler.args {
        if let FlowArgument::Ref { value_ref } = argument {
            validate_ref(value_ref, known)?;
        }
    }
    Ok(())
}

fn validate_ref(
    value_ref: &FlowValueRef,
    known: &impl Fn(&str) -> bool,
) -> Result<(), FlowValidationError> {
    require_variable("value reference", &value_ref.root)?;
    if !known(&value_ref.root)
        || value_ref.path.len() > MAX_VALUE_REF_PATH
        || value_ref.path.iter().any(|part| !is_identifier(part))
    {
        return Err(FlowValidationError::new(format!(
            "unknown or invalid interaction-flow value reference: {}",
            value_ref.root
        )));
    }
    Ok(())
}

fn validate_resolver_graph(
    resolvers: &HashMap<String, FlowValueResolver>,
) -> Result<(), FlowValidationError> {
    fn visit<'a>(
        name: &'a str,
        resolvers: &'a HashMap<String, FlowValueResolver>,
        visiting: &mut HashSet<&'a str>,
        visited: &mut HashSet<&'a str>,
    ) -> Result<(), FlowValidationError> {
        if visited.contains(name) {
            return Ok(());
        }
        if !visiting.insert(name) {
            return Err(FlowValidationError::new(
                "interaction-flow resolver dependency cycle",
            ));
        }
        let resolver = resolvers
            .get(name)
            .ok_or_else(|| FlowValidationError::new("interaction-flow resolver is missing"))?;
        for dependency in resolver
            .handler
            .args
            .iter()
            .filter_map(|argument| match argument {
                FlowArgument::Ref { value_ref } if resolvers.contains_key(&value_ref.root) => {
                    Some(value_ref.root.as_str())
                }
                _ => None,
            })
        {
            visit(dependency, resolvers, visiting, visited)?;
        }
        visiting.remove(name);
        visited.insert(name);
        Ok(())
    }

    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();
    for name in resolvers.keys() {
        visit(name, resolvers, &mut visiting, &mut visited)?;
    }
    Ok(())
}

fn require_identifier(kind: &str, value: &str) -> Result<(), FlowValidationError> {
    if value.is_empty()
        || value.len() > 128
        || !value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.' | '/')
        })
    {
        return Err(FlowValidationError::new(format!(
            "invalid interaction-flow {kind}: {value}"
        )));
    }
    Ok(())
}

fn require_variable(kind: &str, value: &str) -> Result<(), FlowValidationError> {
    let Some(name) = value.strip_prefix('$') else {
        return Err(FlowValidationError::new(format!(
            "invalid interaction-flow {kind}: {value}"
        )));
    };
    if name.is_empty()
        || name.len() > 127
        || !name
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_alphabetic() || character == '_')
        || !name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
    {
        return Err(FlowValidationError::new(format!(
            "invalid interaction-flow {kind}: {value}"
        )));
    }
    Ok(())
}

fn is_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_alphabetic() || character == '_')
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
}

fn is_builtin(value: &str) -> bool {
    matches!(value, "$env" | "$context")
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FlowDefinitionRequest {
    pub flow_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FlowDefinitionResponse {
    pub definition_revision: String,
    pub source_session_id: String,
    pub definition: InteractionFlowDefinition,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FlowExecuteRequest {
    pub flow_id: String,
    pub source_session_id: String,
    pub definition_revision: String,
    pub operation: String,
    pub operation_id: String,
    #[serde(default)]
    pub args: Vec<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowExecuteResponse {
    pub data: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FlowCloseReason {
    Completed,
    Cancelled,
    Expired,
    StartFailed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FlowCloseRequest {
    pub flow_id: String,
    pub source_session_id: String,
    pub operation_id: String,
    pub reason: FlowCloseReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowCloseResponse {
    pub closed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn definition() -> InteractionFlowDefinition {
        InteractionFlowDefinition {
            protocol_version: PROTOCOL_VERSION,
            flow_id: "provider.connect".into(),
            start_step_id: "credentials".into(),
            steps: HashMap::from([(
                "credentials".into(),
                InteractionFlowStep {
                    id: "credentials".into(),
                    content: FlowStepContent::Form {
                        presentation: FlowPresentation {
                            renderer: FlowRenderer::TypedForm,
                            title: Some("Connect".into()),
                            description: None,
                            content: None,
                        },
                        form: FlowForm {
                            fields: vec![FlowFormField {
                                var: "$host".into(),
                                value_type: FlowFieldValueType::String,
                                control: FlowFieldControl::Text,
                                required: true,
                                repeat: false,
                                label: Some("Host".into()),
                                default_value: None,
                                default_value_ref: None,
                                options: None,
                                options_ref: None,
                                option_unavailable_text: None,
                                read_only: false,
                                min_rows: None,
                            }],
                        },
                        render_refs: Vec::new(),
                    },
                    transition: FlowTransition::Finish,
                },
            )]),
            resolvers: HashMap::new(),
            lifecycle: FlowLifecycleHandlers {
                complete: Some(FlowHandler {
                    operation: "provider.save".into(),
                    args: vec![FlowArgument::Ref {
                        value_ref: FlowValueRef {
                            root: "$host".into(),
                            path: Vec::new(),
                        },
                    }],
                }),
                ..FlowLifecycleHandlers::default()
            },
        }
    }

    #[test]
    fn canonical_definition_round_trips_and_validates() {
        let definition = definition();
        definition.validate().unwrap();
        let value = serde_json::to_value(&definition).unwrap();
        assert_eq!(value["lifecycle"]["complete"]["operation"], "provider.save");
        assert!(value.to_string().contains("\"type\":\"ref\""));
        assert_eq!(
            serde_json::from_value::<InteractionFlowDefinition>(value).unwrap(),
            definition
        );
    }

    #[test]
    fn dependencies_are_inferred_from_typed_arguments_and_cycles_are_rejected() {
        let mut definition = definition();
        definition.resolvers.insert(
            "$a".into(),
            FlowValueResolver {
                output_var: "$a".into(),
                handler: FlowHandler {
                    operation: "resolve.a".into(),
                    args: vec![FlowArgument::Ref {
                        value_ref: FlowValueRef {
                            root: "$b".into(),
                            path: Vec::new(),
                        },
                    }],
                },
            },
        );
        definition.resolvers.insert(
            "$b".into(),
            FlowValueResolver {
                output_var: "$b".into(),
                handler: FlowHandler {
                    operation: "resolve.b".into(),
                    args: vec![FlowArgument::Ref {
                        value_ref: FlowValueRef {
                            root: "$a".into(),
                            path: Vec::new(),
                        },
                    }],
                },
            },
        );
        assert!(definition
            .validate()
            .unwrap_err()
            .to_string()
            .contains("cycle"));
    }

    #[test]
    fn node_routes_and_requests_do_not_accept_routing_identity() {
        assert_eq!(
            definition_target("camera"),
            "/camera/interaction-flow/definition"
        );
        assert_eq!(execute_target("camera"), "/camera/interaction-flow/execute");
        assert_eq!(close_target("camera"), "/camera/interaction-flow/close");
        let request = serde_json::from_value::<FlowExecuteRequest>(json!({
            "flowId": "provider.connect",
            "sourceSessionId": "source-1",
            "definitionRevision": "sha256:abc",
            "operation": "provider.discover",
            "operationId": "operation-1",
            "args": [],
            "nodeId": "untrusted"
        }));
        assert!(request.is_err());
    }

    #[test]
    fn node_renderer_is_typed_form_only_on_the_wire() {
        let mut definition = definition();
        let step = definition.steps.get_mut("credentials").unwrap();
        let FlowStepContent::Form { presentation, .. } = &mut step.content else {
            unreachable!();
        };
        presentation.renderer = FlowRenderer::CustomConnectMdx;
        assert!(serde_json::to_value(definition).is_err());
    }

    #[test]
    fn input_shape_is_stable_across_steps_and_passwords_are_user_owned() {
        let mut invalid_definition = definition();
        invalid_definition.steps.insert(
            "second".into(),
            InteractionFlowStep {
                id: "second".into(),
                content: FlowStepContent::Form {
                    presentation: FlowPresentation {
                        renderer: FlowRenderer::TypedForm,
                        title: None,
                        description: None,
                        content: None,
                    },
                    form: FlowForm {
                        fields: vec![FlowFormField {
                            var: "$host".into(),
                            value_type: FlowFieldValueType::Boolean,
                            control: FlowFieldControl::Boolean,
                            required: true,
                            repeat: false,
                            label: None,
                            default_value: None,
                            default_value_ref: None,
                            options: None,
                            options_ref: None,
                            option_unavailable_text: None,
                            read_only: false,
                            min_rows: None,
                        }],
                    },
                    render_refs: Vec::new(),
                },
                transition: FlowTransition::Finish,
            },
        );
        assert!(invalid_definition.validate().is_err());

        let mut definition = definition();
        let step = definition.steps.get_mut("credentials").unwrap();
        let FlowStepContent::Form { form, .. } = &mut step.content else {
            unreachable!();
        };
        let field = form.fields.first_mut().unwrap();
        field.control = FlowFieldControl::Password;
        field.default_value = Some(json!("not-allowed"));
        assert!(definition.validate().is_err());
    }

    #[test]
    fn conformance_fixture_matches_the_published_node_schema() {
        let schema: Value = serde_json::from_str(NODE_PROTOCOL_SCHEMA).unwrap();
        let fixture: Value = serde_json::from_str(include_str!(
            "../fixtures/interaction-flow-node.conformance.json"
        ))
        .unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        let errors = validator
            .iter_errors(&fixture)
            .map(|error| error.to_string())
            .collect::<Vec<_>>();
        assert!(errors.is_empty(), "fixture failed: {errors:?}");
    }
}
