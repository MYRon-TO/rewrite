use std::io;

use thiserror::Error;

pub type Result<T, E = RewriteError> = std::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum RewriteError {
    #[error(transparent)]
    IrParse(#[from] IrParseError),

    #[error(transparent)]
    Validation(#[from] ValidationError),

    #[error(transparent)]
    Runtime(#[from] RuntimeError),

    #[error(transparent)]
    Output(#[from] OutputError),
}

impl RewriteError {
    pub fn category(&self) -> ErrorCategory {
        match self {
            Self::IrParse(_) => ErrorCategory::IrParse,
            Self::Validation(_) => ErrorCategory::Validation,
            Self::Runtime(_) => ErrorCategory::Runtime,
            Self::Output(_) => ErrorCategory::Output,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    IrParse,
    Validation,
    Runtime,
    Output,
}

#[derive(Debug, Error)]
pub enum IrParseError {
    #[error("failed to read IR file `{path}`: {source}")]
    Read {
        path: String,
        #[source]
        source: io::Error,
    },

    #[error("failed to parse JSON IR file `{path}`: {source}")]
    Json {
        path: String,
        #[source]
        source: serde_json::Error,
    },

    #[error("unsupported IR format for `{path}`; expected {expected}")]
    UnsupportedFormat { path: String, expected: String },

    #[error("invalid IR at `{location}`: {message}")]
    Invalid { location: String, message: String },
}

impl IrParseError {
    pub fn read(path: impl Into<String>, source: io::Error) -> Self {
        Self::Read {
            path: path.into(),
            source,
        }
    }

    pub fn json(path: impl Into<String>, source: serde_json::Error) -> Self {
        Self::Json {
            path: path.into(),
            source,
        }
    }

    pub fn unsupported_format(path: impl Into<String>, expected: impl Into<String>) -> Self {
        Self::UnsupportedFormat {
            path: path.into(),
            expected: expected.into(),
        }
    }

    pub fn invalid(location: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Invalid {
            location: location.into(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("missing {target_kind} reference `{target_id}` from {owner}.{field}")]
    MissingReference {
        owner: String,
        field: String,
        target_kind: String,
        target_id: String,
    },

    #[error("invalid value at {location}.{field}: {message}")]
    InvalidValue {
        location: String,
        field: String,
        message: String,
    },

    #[error("invalid machine `{machine_id}`: {message}")]
    InvalidMachine { machine_id: String, message: String },

    #[error(
        "definition boundary drift at {location}: field `{field}` is not supported in MVP ({message})"
    )]
    Drift {
        location: String,
        field: String,
        message: String,
    },
}

impl ValidationError {
    pub fn missing_reference(
        owner: impl Into<String>,
        field: impl Into<String>,
        target_kind: impl Into<String>,
        target_id: impl Into<String>,
    ) -> Self {
        Self::MissingReference {
            owner: owner.into(),
            field: field.into(),
            target_kind: target_kind.into(),
            target_id: target_id.into(),
        }
    }

    pub fn invalid_value(
        location: impl Into<String>,
        field: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::InvalidValue {
            location: location.into(),
            field: field.into(),
            message: message.into(),
        }
    }

    pub fn invalid_machine(machine_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::InvalidMachine {
            machine_id: machine_id.into(),
            message: message.into(),
        }
    }

    pub fn drift(
        location: impl Into<String>,
        field: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::Drift {
            location: location.into(),
            field: field.into(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("context variable `{scope}.{name}` is missing at {location}")]
    MissingContextVariable {
        scope: String,
        name: String,
        location: String,
    },

    #[error("unknown generator `{generator_id}` at {location}")]
    UnknownGenerator {
        generator_id: String,
        location: String,
    },

    #[error("machine `{machine_id}` got stuck at state `{state_id}`: {message}")]
    MachineStuck {
        machine_id: String,
        state_id: String,
        message: String,
    },

    #[error("machine `{machine_id}` exceeded max steps {max_steps}")]
    StepLimitExceeded {
        machine_id: String,
        max_steps: usize,
    },

    #[error("invalid context value at {location}: {message}")]
    InvalidContextValue { location: String, message: String },
    #[error("invalid time profile `{profile_id}` at {field}: {message}")]
    InvalidTimeProfile {
        profile_id: String,
        field: String,
        message: String,
    },

    #[error("invalid value expression at {location}.{field}: {message}")]
    InvalidValueExpression {
        location: String,
        field: String,
        message: String,
    },
}

impl RuntimeError {
    pub fn missing_context_variable(
        scope: impl Into<String>,
        name: impl Into<String>,
        location: impl Into<String>,
    ) -> Self {
        Self::MissingContextVariable {
            scope: scope.into(),
            name: name.into(),
            location: location.into(),
        }
    }

    pub fn unknown_generator(generator_id: impl Into<String>, location: impl Into<String>) -> Self {
        Self::UnknownGenerator {
            generator_id: generator_id.into(),
            location: location.into(),
        }
    }

    pub fn machine_stuck(
        machine_id: impl Into<String>,
        state_id: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::MachineStuck {
            machine_id: machine_id.into(),
            state_id: state_id.into(),
            message: message.into(),
        }
    }

    pub fn step_limit_exceeded(machine_id: impl Into<String>, max_steps: usize) -> Self {
        Self::StepLimitExceeded {
            machine_id: machine_id.into(),
            max_steps,
        }
    }

    pub fn invalid_context_value(location: impl Into<String>, message: impl Into<String>) -> Self {
        Self::InvalidContextValue {
            location: location.into(),
            message: message.into(),
        }
    }

    pub fn invalid_time_profile(
        profile_id: impl Into<String>,
        field: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::InvalidTimeProfile {
            profile_id: profile_id.into(),
            field: field.into(),
            message: message.into(),
        }
    }

    pub fn invalid_value_expression(
        location: impl Into<String>,
        field: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::InvalidValueExpression {
            location: location.into(),
            field: field.into(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Error)]
pub enum OutputError {
    #[error("failed to create output directory `{path}`: {source}")]
    CreateDir {
        path: String,
        #[source]
        source: io::Error,
    },

    #[error("failed to write output file `{path}`: {source}")]
    Write {
        path: String,
        #[source]
        source: io::Error,
    },

    #[error("failed to serialize output record at {location}: {source}")]
    Serialize {
        location: String,
        #[source]
        source: serde_json::Error,
    },
}

impl OutputError {
    pub fn create_dir(path: impl Into<String>, source: io::Error) -> Self {
        Self::CreateDir {
            path: path.into(),
            source,
        }
    }

    pub fn write(path: impl Into<String>, source: io::Error) -> Self {
        Self::Write {
            path: path.into(),
            source,
        }
    }

    pub fn serialize(location: impl Into<String>, source: serde_json::Error) -> Self {
        Self::Serialize {
            location: location.into(),
            source,
        }
    }
}
