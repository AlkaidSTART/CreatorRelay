use super::*;

#[derive(Debug)]
pub(crate) struct CoreToolExecution {
    pub(crate) status: String,
    pub(crate) result: Value,
}

pub(crate) fn success_result(result: Value) -> CoreToolExecution {
    CoreToolExecution {
        status: "success".to_string(),
        result,
    }
}
