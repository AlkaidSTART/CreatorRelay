use super::*;

#[derive(Debug)]
pub(super) struct CoreToolExecution {
    pub(super) status: String,
    pub(super) result: Value,
}

pub(super) fn success_result(result: Value) -> CoreToolExecution {
    CoreToolExecution {
        status: "success".to_string(),
        result,
    }
}
